pub fn assert_eq<L: ToString, R: ToString>(left: L, right: R) {
    let left = left.to_string();
    let right = right.to_string();

    #[cfg(windows)]
    similar_asserts::assert_eq!(left.replace("\r\n", "\n"), right.replace("\r\n", "\n"));

    #[cfg(not(windows))]
    similar_asserts::assert_eq!(left, right);
}
