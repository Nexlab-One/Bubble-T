#[test]
fn parse_keyboard_enhancements_response() {
    let input = b"\x1b[?3u";
    let (msgs, n) = bubble_t::query_parser::parse_responses(input);
    assert_eq!(n, input.len());
    let msg = msgs[0]
        .downcast_ref::<bubble_t::KeyboardEnhancementsMsg>()
        .unwrap();
    assert_eq!(msg.flags, 3);
    assert!(msg.supports_key_disambiguation());
    assert!(msg.supports_event_types());
}
