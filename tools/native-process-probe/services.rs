use super::native_memory::{PageTable, copy_from_user, copy_to_user};

// ------------------------=
// FUNC: client_contract
// DESC: Verifies actual user-mode compiler and window client marshaling with checked copies and a denial/success sequence.
// ------------------=
pub fn client_contract<T: PageTable>(memory: &mut T, operation: u64, args: [u64; 6], calls: &mut u64) -> Option<u64> {
    if operation == 0x121c {
        assert!(*calls < 2);
        assert_eq!(args[0], 2);
        assert_eq!(args[2], 8);
        let mut bytes = [0u8; 8];
        copy_from_user(memory, args[1], &mut bytes).unwrap();
        assert_eq!(u64::from_le_bytes(bytes), 0x123456789abcdef0);
        *calls += 1;
        if *calls == 1 { return Some((-13i64) as u64); }
        copy_to_user(memory, args[3], &8u64.to_le_bytes()).unwrap();
        return Some(0);
    }
    if operation == 0x1100 {
        assert_eq!(*calls, 2);
        assert_eq!(args, [91, 7, 10, 20, 300, 200]);
        *calls += 1;
        return Some(27);
    }
    None
}
