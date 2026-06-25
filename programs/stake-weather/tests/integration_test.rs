#[cfg(test)]
mod integration_tests {
    use litesvm::LiteSVM;
    use solana_account::AccountSharedData;
    use solana_keypair::Keypair;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use solana_transaction::Transaction;
    use solana_instruction::{AccountMeta, Instruction};
    use std::str::FromStr;
    use solana_clock::Clock;

    const PROGRAM_ID: &str = "Asn5AeENGV3LMtZKjf3sWectSeFKif2Ea5FZD3E8Lxc5";
    const SWITCHBOARD_PROGRAM_ID: &str = "orac1eFjzWL5R3RbbdMV68K9H6TaCVVcL6LjvQQWAbz";
    const SYSTEM_PROGRAM_ID: &str = "11111111111111111111111111111111";

    const MUMBAI_FEED_HASH: [u8; 32] = [
        0x13, 0x71, 0x6a, 0xbd, 0x2e, 0x71, 0x9f, 0x65,
        0x2c, 0x0f, 0x4a, 0x03, 0x7a, 0xcf, 0xf7, 0xc9,
        0x45, 0xd6, 0x2b, 0xc9, 0x6e, 0xbb, 0x6b, 0x42,
        0x24, 0xe2, 0x19, 0x28, 0xd8, 0x8b, 0x69, 0xb0,
    ];

    const DELHI_FEED_HASH: [u8; 32] = [
        0x8d, 0x63, 0x29, 0x76, 0x58, 0xea, 0xbe, 0xdc,
        0x0e, 0x91, 0x37, 0x80, 0x0f, 0xfa, 0x97, 0x9b,
        0x17, 0x03, 0xf7, 0xa9, 0xb6, 0x30, 0x0b, 0x4c,
        0xf7, 0x88, 0xb1, 0x20, 0xb2, 0x6e, 0x7c, 0x79,
    ];

    const DISC_CREATE_BET: [u8; 8] = [197, 42, 153, 2, 59, 63, 143, 246];
    const DISC_JOIN_BET: [u8; 8] = [69, 116, 82, 26, 144, 192, 58, 238];
    const DISC_SETTLE_BET: [u8; 8] = [115, 55, 234, 177, 227, 4, 10, 67];
    const DISC_CANCEL_BET: [u8; 8] = [17, 248, 130, 128, 153, 227, 231, 9];

    fn program_id() -> Pubkey { Pubkey::from_str(PROGRAM_ID).unwrap() }
    fn switchboard_id() -> Pubkey { Pubkey::from_str(SWITCHBOARD_PROGRAM_ID).unwrap() }
    fn system_program_id() -> Pubkey { Pubkey::from_str(SYSTEM_PROGRAM_ID).unwrap() }

    fn bet_pda(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"bet", creator.as_ref()], &program_id())
    }

    fn vault_pda(creator: &Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[b"vault", creator.as_ref()], &program_id())
    }

    struct BetAccount {
        creator: Pubkey,
        challenger: Pubkey,
        city: u8,
        threshold: i32,
        direction: bool,
        deadline: i64,
        lamports: u64,
        settled: bool,
        bump: u8,
        vault_bump: u8,
    }

    fn parse_bet_account(data: &[u8]) -> BetAccount {
        assert!(data.len() >= 97, "bet account data too short: {} bytes", data.len());
        let mut o = 8;

        let creator = Pubkey::from(<[u8; 32]>::try_from(&data[o..o+32]).unwrap()); o += 32;
        let challenger = Pubkey::from(<[u8; 32]>::try_from(&data[o..o+32]).unwrap()); o += 32;
        let city = data[o]; o += 1;
        let threshold = i32::from_le_bytes(data[o..o+4].try_into().unwrap()); o += 4;
        let direction = data[o] != 0; o += 1;
        let deadline = i64::from_le_bytes(data[o..o+8].try_into().unwrap()); o += 8;
        let lamports = u64::from_le_bytes(data[o..o+8].try_into().unwrap()); o += 8;
        let settled = data[o] != 0; o += 1;
        let bump = data[o]; o += 1;
        let vault_bump = data[o];

        BetAccount { creator, challenger, city, threshold, direction, deadline, lamports, settled, bump, vault_bump }
    }

    fn mock_oracle_data(feeds: &[([u8; 32], i128)]) -> Vec<u8> {
        const FEED_SIZE: usize = 49;
        let mut feeds_bytes = Vec::new();
        for (hash, temp_times_10) in feeds {
            let raw: i128 = temp_times_10 * 100_000_000_000_000_000i128;
            let mut entry = [0u8; FEED_SIZE];
            entry[..32].copy_from_slice(hash);
            entry[32..48].copy_from_slice(&raw.to_le_bytes());
            entry[48] = 1;
            feeds_bytes.extend_from_slice(&entry);
        }
        let mut quote_body = vec![0u8; 46];
        quote_body.extend_from_slice(&feeds_bytes);
        let data_len = (quote_body.len() as u16).to_le_bytes();
        let mut account = vec![0u8; 40];
        account.extend_from_slice(&data_len);
        account.extend_from_slice(&quote_body);
        account
    }

    fn set_oracle(svm: &mut LiteSVM, oracle: &Keypair, feeds: &[([u8; 32], i128)]) {
        let data = mock_oracle_data(feeds);
        let mut account = AccountSharedData::new(1_000_000, data.len(), &switchboard_id());
        account.set_data_from_slice(&data);
        svm.set_account(oracle.pubkey(), account.into()).unwrap();
    }

    fn encode_create_bet(city: u8, threshold: i32, direction: bool, deadline: i64, lamports: u64) -> Vec<u8> {
        let mut data = DISC_CREATE_BET.to_vec();
        data.push(city);
        data.extend_from_slice(&threshold.to_le_bytes());
        data.push(direction as u8);
        data.extend_from_slice(&deadline.to_le_bytes());
        data.extend_from_slice(&lamports.to_le_bytes());
        data
    }

    fn warp_past_deadline(svm: &mut LiteSVM) {
        let mut clock = svm.get_sysvar::<Clock>();
        clock.unix_timestamp = 9_999_999_999i64;
        svm.set_sysvar::<Clock>(&clock);
    }

    fn load_svm() -> LiteSVM {
        let mut svm = LiteSVM::new();
        svm.add_program_from_file(
            program_id(),
            "../../target/deploy/stake_weather.so",
        )
        .expect("failed to load stake_weather.so — run `anchor build` first");
        svm
    }

    fn fund(svm: &mut LiteSVM, pubkey: &Pubkey, lamports: u64) {
        let account = AccountSharedData::new(lamports, 0, &system_program_id());
        svm.set_account(*pubkey, account.into()).unwrap();
    }

    fn do_create_bet(
        svm: &mut LiteSVM,
        creator: &Keypair,
        city: u8,
        threshold: i32,
        direction: bool,
        deadline: i64,
        stake: u64,
    ) {
        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(bet_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(system_program_id(), false),
            ],
            data: encode_create_bet(city, threshold, direction, deadline, stake),
        };
        let bh = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[ix], Some(&creator.pubkey()), &[creator], bh,
        )).expect("create_bet failed");
    }

    fn do_join_bet(svm: &mut LiteSVM, creator: &Keypair, challenger: &Keypair) {
        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(bet_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new_readonly(creator.pubkey(), false),
                AccountMeta::new(challenger.pubkey(), true),
                AccountMeta::new_readonly(system_program_id(), false),
            ],
            data: DISC_JOIN_BET.to_vec(),
        };
        let bh = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[ix], Some(&challenger.pubkey()), &[challenger], bh,
        )).expect("join_bet failed");
    }

    fn do_settle_bet(
        svm: &mut LiteSVM,
        creator: &Keypair,
        challenger: &Keypair,
        oracle: &Keypair,
    ) -> litesvm::types::TransactionResult {
        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(bet_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(creator.pubkey(), false),
                AccountMeta::new(challenger.pubkey(), false),
                AccountMeta::new_readonly(oracle.pubkey(), false),
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(system_program_id(), false),
            ],
            data: DISC_SETTLE_BET.to_vec(),
        };
        let bh = svm.latest_blockhash();
        svm.send_transaction(Transaction::new_signed_with_payer(
            &[ix], Some(&creator.pubkey()), &[creator], bh,
        ))
    }

    fn winner_from_logs(logs: &[String]) -> Option<String> {
        for log in logs {
            if log.contains("Winner: creator") { return Some("creator".into()); }
            if log.contains("Winner: challenger") { return Some("challenger".into()); }
        }
        None
    }

    #[test]
    fn test_create_bet_fields() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);

        let deadline = 1_000_000_000i64;
        let stake = 1_000_000u64;
        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let (vault_pda, _) = vault_pda(&creator.pubkey());

        do_create_bet(&mut svm, &creator, 0, 300, true, deadline, stake);

        let raw = svm.get_account(&bet_pda).unwrap();
        let bet = parse_bet_account(&raw.data);
        assert_eq!(bet.creator, creator.pubkey(), "creator mismatch");
        assert_eq!(bet.challenger, Pubkey::default(), "challenger should be unset");
        assert_eq!(bet.city, 0, "city should be Mumbai (0)");
        assert_eq!(bet.threshold, 300, "threshold mismatch");
        assert!(bet.direction, "direction should be true (above)");
        assert_eq!(bet.deadline, deadline, "deadline mismatch");
        assert_eq!(bet.lamports, stake, "lamports mismatch");
        assert!(!bet.settled, "bet should not be settled yet");

        let vault = svm.get_account(&vault_pda).unwrap();
        assert!(vault.lamports >= stake, "vault holds less than stake");
    }

    #[test]
    fn test_join_bet_vault_holds_both_stakes() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        let stake = 1_000_000u64;
        do_create_bet(&mut svm, &creator, 0, 300, true, 1_000_000_000, stake);
        do_join_bet(&mut svm, &creator, &challenger);

        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let raw = svm.get_account(&bet_pda).unwrap();
        let bet = parse_bet_account(&raw.data);
        assert_eq!(bet.challenger, challenger.pubkey(), "challenger not recorded");

        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let vault = svm.get_account(&vault_pda).unwrap();
        assert!(vault.lamports >= stake * 2, "vault should hold at least 2x stake");
    }

    #[test]
    fn test_settle_creator_wins() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        let stake = 1_000_000u64;
        do_create_bet(&mut svm, &creator, 0, 260, true, 1i64, stake);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);
        set_oracle(&mut svm, &oracle, &[(MUMBAI_FEED_HASH, 282)]);

        let creator_before = svm.get_account(&creator.pubkey()).unwrap().lamports;
        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_ok(), "settle_bet failed: {:?}", result.err());

        let logs = result.unwrap().logs;
        assert_eq!(winner_from_logs(&logs).as_deref(), Some("creator"), "expected creator to win");

        let creator_after = svm.get_account(&creator.pubkey()).unwrap().lamports;
        assert!(creator_after >= creator_before + stake, "creator did not gain at least their stake");

        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let vault_lamports = svm.get_account(&vault_pda).map(|a| a.lamports).unwrap_or(0);
        assert_eq!(vault_lamports, 0, "vault should be empty after settle");
    }

    #[test]
    fn test_settle_challenger_wins() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        let stake = 1_000_000u64;
        do_create_bet(&mut svm, &creator, 0, 260, false, 1i64, stake);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);
        set_oracle(&mut svm, &oracle, &[(MUMBAI_FEED_HASH, 282)]);

        let challenger_before = svm.get_account(&challenger.pubkey()).unwrap().lamports;
        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_ok(), "settle_bet failed: {:?}", result.err());

        let logs = result.unwrap().logs;
        assert_eq!(winner_from_logs(&logs).as_deref(), Some("challenger"), "expected challenger to win");

        let challenger_after = svm.get_account(&challenger.pubkey()).unwrap().lamports;
        assert!(challenger_after >= challenger_before + stake, "challenger did not gain at least their stake");
    }

    #[test]
    fn test_temperature_read_from_oracle_account() {
        let mut svm = load_svm();

        let creator_a = Keypair::new();
        let creator_b = Keypair::new();
        let challenger_a = Keypair::new();
        let challenger_b = Keypair::new();
        let oracle_a = Keypair::new();
        let oracle_b = Keypair::new();

        for kp in [&creator_a, &creator_b, &challenger_a, &challenger_b] {
            fund(&mut svm, &kp.pubkey(), 10_000_000);
        }

        let stake = 1_000_000u64;
        do_create_bet(&mut svm, &creator_a, 0, 260, true, 1i64, stake);
        do_join_bet(&mut svm, &creator_a, &challenger_a);
        do_create_bet(&mut svm, &creator_b, 0, 260, true, 1i64, stake);
        do_join_bet(&mut svm, &creator_b, &challenger_b);

        warp_past_deadline(&mut svm);

        // oracle_a: 28.2°C → 282 >= 260 → creator wins
        set_oracle(&mut svm, &oracle_a, &[(MUMBAI_FEED_HASH, 282)]);
        // oracle_b: 24.0°C → 240 < 260 → challenger wins
        set_oracle(&mut svm, &oracle_b, &[(MUMBAI_FEED_HASH, 240)]);

        let result_a = do_settle_bet(&mut svm, &creator_a, &challenger_a, &oracle_a);
        assert!(result_a.is_ok(), "settle A failed: {:?}", result_a.err());

        let result_b = do_settle_bet(&mut svm, &creator_b, &challenger_b, &oracle_b);
        assert!(result_b.is_ok(), "settle B failed: {:?}", result_b.err());

        assert_eq!(
            winner_from_logs(&result_a.unwrap().logs).as_deref(), Some("creator"),
            "oracle_a (28.2°C) should give creator the win"
        );
        assert_eq!(
            winner_from_logs(&result_b.unwrap().logs).as_deref(), Some("challenger"),
            "oracle_b (24.0°C) should give challenger the win"
        );
    }

    #[test]
    fn test_settle_before_deadline_fails() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        do_create_bet(&mut svm, &creator, 0, 260, true, 9_999_999_999i64, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);
        set_oracle(&mut svm, &oracle, &[(MUMBAI_FEED_HASH, 282)]);

        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_err(), "settle_bet should fail before deadline");
    }

    #[test]
    fn test_fake_oracle_rejected() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let fake_oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        do_create_bet(&mut svm, &creator, 0, 260, true, 1i64, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);

        let fake_data = mock_oracle_data(&[(MUMBAI_FEED_HASH, 282)]);
        let mut fake_account = AccountSharedData::new(1_000_000, fake_data.len(), &system_program_id());
        fake_account.set_data_from_slice(&fake_data);
        svm.set_account(fake_oracle.pubkey(), fake_account.into()).unwrap();

        let result = do_settle_bet(&mut svm, &creator, &challenger, &fake_oracle);
        assert!(result.is_err(), "fake oracle (wrong owner) should be rejected");
    }

    #[test]
    fn test_wrong_feed_hash_rejected() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        do_create_bet(&mut svm, &creator, 0, 260, true, 1i64, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);
        set_oracle(&mut svm, &oracle, &[(DELHI_FEED_HASH, 282)]);

        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_err(), "oracle with wrong feed hash should be rejected");
    }

    #[test]
    fn test_correct_feed_found_among_multiple() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        do_create_bet(&mut svm, &creator, 0, 260, true, 1i64, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);

        set_oracle(&mut svm, &oracle, &[
            (DELHI_FEED_HASH, 100),
            (MUMBAI_FEED_HASH, 282),
        ]);

        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_ok(), "should find Mumbai feed at index 1: {:?}", result.err());

        let logs = result.unwrap().logs;
        assert_eq!(winner_from_logs(&logs).as_deref(), Some("creator"),
            "creator should win using Mumbai temp 28.2°C ≥ 26°C");
    }

    #[test]
    fn test_temperature_at_exact_threshold_boundary() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        do_create_bet(&mut svm, &creator, 0, 280, true, 1i64, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);
        set_oracle(&mut svm, &oracle, &[(MUMBAI_FEED_HASH, 280)]);

        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_ok(), "settle failed at boundary: {:?}", result.err());

        let logs = result.unwrap().logs;
        assert_eq!(winner_from_logs(&logs).as_deref(), Some("creator"),
            "temp == threshold with direction=above should give creator the win");
    }

    #[test]
    fn test_cancel_bet_refunds_creator() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);

        let stake = 1_000_000u64;
        do_create_bet(&mut svm, &creator, 0, 300, true, 1_000_000_000, stake);

        let balance_before = svm.get_account(&creator.pubkey()).unwrap().lamports;

        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let cancel_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(bet_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(system_program_id(), false),
            ],
            data: DISC_CANCEL_BET.to_vec(),
        };
        let bh = svm.latest_blockhash();
        let result = svm.send_transaction(Transaction::new_signed_with_payer(
            &[cancel_ix], Some(&creator.pubkey()), &[&creator], bh,
        ));
        assert!(result.is_ok(), "cancel_bet failed: {:?}", result.err());

        let balance_after = svm.get_account(&creator.pubkey()).unwrap().lamports;
        assert!(balance_after >= balance_before + stake,
            "creator should recover at least their stake on cancel");
    }

    #[test]
    fn test_cancel_after_challenger_fails() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        do_create_bet(&mut svm, &creator, 0, 300, true, 1_000_000_000, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);

        let (bet_pda, _) = bet_pda(&creator.pubkey());
        let (vault_pda, _) = vault_pda(&creator.pubkey());
        let cancel_ix = Instruction {
            program_id: program_id(),
            accounts: vec![
                AccountMeta::new(bet_pda, false),
                AccountMeta::new(vault_pda, false),
                AccountMeta::new(creator.pubkey(), true),
                AccountMeta::new_readonly(system_program_id(), false),
            ],
            data: DISC_CANCEL_BET.to_vec(),
        };
        let bh = svm.latest_blockhash();
        let result = svm.send_transaction(Transaction::new_signed_with_payer(
            &[cancel_ix], Some(&creator.pubkey()), &[&creator], bh,
        ));
        assert!(result.is_err(), "cancel_bet should fail after challenger has joined");
    }

    /// Proves the full chain: oracle account is owned by Switchboard → program reads
    /// temperature from that account → settlement outcome matches the account data.
    /// The temperature in the logs is not hardcoded — it comes from the oracle account bytes.
    #[test]
    fn test_temperature_comes_from_switchboard_owned_account() {
        let mut svm = load_svm();
        let creator = Keypair::new();
        let challenger = Keypair::new();
        let oracle = Keypair::new();
        fund(&mut svm, &creator.pubkey(), 10_000_000);
        fund(&mut svm, &challenger.pubkey(), 10_000_000);

        let temp_times_10: i128 = 267; // 26.7°C

        do_create_bet(&mut svm, &creator, 0, 260, true, 1i64, 1_000_000);
        do_join_bet(&mut svm, &creator, &challenger);
        warp_past_deadline(&mut svm);
        set_oracle(&mut svm, &oracle, &[(MUMBAI_FEED_HASH, temp_times_10)]);

        // 1. Verify oracle account is owned by Switchboard before settling.
        let oracle_account = svm.get_account(&oracle.pubkey()).unwrap();
        assert_eq!(
            oracle_account.owner,
            switchboard_id(),
            "oracle account must be owned by Switchboard program"
        );

        // 2. Settle — the program reads temperature from the oracle account data.
        let result = do_settle_bet(&mut svm, &creator, &challenger, &oracle);
        assert!(result.is_ok(), "settle failed: {:?}", result.err());

        // 3. Verify the temperature in the logs matches what we stored in the account.
        let logs = result.unwrap().logs;
        let temp_log = logs.iter().find(|l| l.contains("Temperature")).expect("no temperature log");
        assert!(
            temp_log.contains("26.7"),
            "log should show 26.7°C (the value from the oracle account), got: {}",
            temp_log
        );
        assert_eq!(
            winner_from_logs(&logs).as_deref(), Some("creator"),
            "26.7°C >= 26.0°C threshold → creator wins"
        );
    }
}
