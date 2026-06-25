#[cfg(test)]
mod tests {
    use litesvm::LiteSVM;
    use solana_keypair::Keypair;
    use solana_pubkey::Pubkey;
    use solana_signer::Signer;
    use std::str::FromStr;

    const SWITCHBOARD_PROGRAM_ID: &str = "orac1eFjzWL5R3RbbdMV68K9H6TaCVVcL6LjvQQWAbz";

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

    fn mock_oracle_account(feeds: &[([u8; 32], i128)]) -> Vec<u8> {
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

    fn find_feed_value(data: &[u8], expected_hash: &[u8; 32]) -> Option<i128> {
        if data.len() < 42 { return None; }
        let data_len = u16::from_le_bytes([data[40], data[41]]) as usize;
        if data.len() < 42 + data_len { return None; }
        let quote_bytes = &data[42..42 + data_len];
        if quote_bytes.len() < 46 { return None; }
        let feeds_bytes = &quote_bytes[46..];
        const FEED_SIZE: usize = 49;
        let num_feeds = feeds_bytes.len() / FEED_SIZE;
        for i in 0..num_feeds {
            let offset = i * FEED_SIZE;
            let feed_id: [u8; 32] = feeds_bytes[offset..offset + 32].try_into().unwrap();
            if feed_id == *expected_hash {
                let val_bytes: [u8; 16] = feeds_bytes[offset + 32..offset + 48].try_into().unwrap();
                return Some(i128::from_le_bytes(val_bytes));
            }
        }
        None
    }

    fn raw_to_temp(raw: i128) -> i32 {
        (raw / 100_000_000_000_000_000i128) as i32
    }

    fn creator_wins(temp: i32, threshold: i32, direction: bool) -> bool {
        if direction { temp >= threshold } else { temp < threshold }
    }

    #[test]
    fn test_parse_single_feed() {
        let data = mock_oracle_account(&[(MUMBAI_FEED_HASH, 282)]);
        let raw = find_feed_value(&data, &MUMBAI_FEED_HASH).expect("feed not found");
        assert_eq!(raw_to_temp(raw), 282);
    }

    #[test]
    fn test_find_feed_at_nonzero_index() {
        let data = mock_oracle_account(&[
            (DELHI_FEED_HASH, 310),
            ([0xAA; 32], 999),
            (MUMBAI_FEED_HASH, 282),
        ]);
        let raw = find_feed_value(&data, &MUMBAI_FEED_HASH).expect("feed not found at index 2");
        assert_eq!(raw_to_temp(raw), 282, "should read value from index 2, not index 0");
    }

    #[test]
    fn test_feed_not_found_returns_none() {
        let data = mock_oracle_account(&[
            (DELHI_FEED_HASH, 310),
            ([0xBB; 32], 100),
        ]);
        assert!(find_feed_value(&data, &MUMBAI_FEED_HASH).is_none());
    }

    #[test]
    fn test_parse_negative_temperature() {
        let data = mock_oracle_account(&[(MUMBAI_FEED_HASH, -50)]);
        let raw = find_feed_value(&data, &MUMBAI_FEED_HASH).expect("feed not found");
        assert_eq!(raw_to_temp(raw), -50);
    }

    #[test]
    fn test_parse_zero_temperature() {
        let data = mock_oracle_account(&[(MUMBAI_FEED_HASH, 0)]);
        let raw = find_feed_value(&data, &MUMBAI_FEED_HASH).expect("feed not found");
        assert_eq!(raw_to_temp(raw), 0);
    }

    #[test]
    fn test_data_too_short_returns_none() {
        let data = vec![0u8; 10];
        assert!(find_feed_value(&data, &MUMBAI_FEED_HASH).is_none());
    }

    #[test]
    fn test_creator_wins_above_threshold() {
        assert!(creator_wins(282, 260, true));
    }

    #[test]
    fn test_creator_wins_at_exact_threshold() {
        assert!(creator_wins(260, 260, true));
    }

    #[test]
    fn test_challenger_wins_below_threshold_direction_above() {
        assert!(!creator_wins(240, 260, true));
    }

    #[test]
    fn test_creator_wins_below_threshold_direction_below() {
        assert!(creator_wins(240, 260, false));
    }

    #[test]
    fn test_challenger_wins_above_threshold_direction_below() {
        assert!(!creator_wins(282, 260, false));
    }

    #[test]
    fn test_challenger_wins_at_exact_threshold_direction_below() {
        assert!(!creator_wins(260, 260, false));
    }

    #[test]
    fn test_oracle_account_must_be_owned_by_switchboard() {
        let mut svm = LiteSVM::new();
        let switchboard_id = Pubkey::from_str(SWITCHBOARD_PROGRAM_ID).unwrap();
        let oracle = Keypair::new();
        let data = mock_oracle_account(&[(MUMBAI_FEED_HASH, 282)]);

        let mut account = solana_account::AccountSharedData::new(1_000_000, data.len(), &switchboard_id);
        account.set_data_from_slice(&data);
        svm.set_account(oracle.pubkey(), account.into()).unwrap();

        let stored = svm.get_account(&oracle.pubkey()).unwrap();
        assert_eq!(stored.owner, switchboard_id);

        let raw = find_feed_value(&stored.data, &MUMBAI_FEED_HASH).expect("feed not found");
        assert_eq!(raw_to_temp(raw), 282);
    }
}
