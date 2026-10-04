use inout::{InOutBuf, InOutBufReserved};

#[test]
fn in_place_read_after_split() {
    let mut storage = [1u8, 2, 3, 4];
    let mut reserved = InOutBufReserved::from_mut_slice(&mut storage, 2).unwrap();
    let (body, suffix) = reserved.split_reserved();
    assert_eq!(body.get_in(), &[1, 2]);
    assert_eq!(suffix, &[3, 4]);
}

#[test]
fn in_place_write_then_read() {
    let mut storage = [1u8, 2, 3, 4];
    let mut reserved = InOutBufReserved::from_mut_slice(&mut storage, 2).unwrap();
    let (mut body, suffix) = reserved.split_reserved();
    body.get_out()[0] = 9;
    assert_eq!(body.get_in()[0], 9);
    suffix[0] = 7;
    assert_eq!(body.get_in()[1], 2);
}

#[test]
fn control_separate_input_output() {
    let input = [1u8, 2];
    let mut output = [0u8; 4];
    let mut reserved = InOutBufReserved::from_slices(&input, &mut output).unwrap();
    let (mut body, suffix) = reserved.split_reserved();
    body.get_out()[0] = 9;
    assert_eq!(body.get_in()[0], 1);
    suffix[0] = 7;
    assert_eq!(body.get_in()[1], 2);
}

#[test]
fn control_in_place_without_reserved_reborrow() {
    let mut storage = [1u8, 2, 3, 4];
    let (head, suffix) = storage.split_at_mut(2);
    let mut body = InOutBuf::from(head);
    body.get_out()[0] = 9;
    assert_eq!(body.get_in()[0], 9);
    suffix[0] = 7;
    assert_eq!(body.get_in()[1], 2);
}

#[test]
fn all_split_boundaries_allow_reborrowing_the_parent() {
    for len in 0..=4 {
        let mut storage = [1u8, 2, 3, 4];
        let mut reserved = InOutBufReserved::from_mut_slice(&mut storage, len).unwrap();
        for _ in 0..2 {
            let (mut body, suffix) = reserved.split_reserved();
            assert_eq!(body.get_in().len(), len);
            body.get_out().fill(9);
            suffix.fill(7);
            assert!(body.get_in().iter().all(|value| *value == 9));
        }
        assert_eq!(&storage[..len], vec![9; len]);
        assert_eq!(&storage[len..], vec![7; 4 - len]);
    }
}

#[test]
fn separate_buffers_preserve_input_at_every_boundary() {
    let input = [1u8, 2, 3, 4];
    for len in 0..=4 {
        let mut output = [0u8; 4];
        let mut reserved =
            InOutBufReserved::from_slices(&input[..len], &mut output).unwrap();
        let (mut body, suffix) = reserved.split_reserved();
        body.get_out().fill(9);
        suffix.fill(7);
        assert_eq!(body.get_in(), &input[..len]);
        assert_eq!(&output[..len], vec![9; len]);
        assert_eq!(&output[len..], vec![7; 4 - len]);
    }
}

#[test]
fn non_copy_values_remain_owned_by_the_original_buffer() {
    let mut storage = [String::from("one"), String::from("two")];
    let mut reserved = InOutBufReserved::from_mut_slice(&mut storage, 1).unwrap();
    let (mut body, suffix) = reserved.split_reserved();
    body.get_out()[0].push('!');
    suffix[0].push('?');
    assert_eq!(body.get_in()[0], "one!");
    assert_eq!(storage[1], "two?");
}

#[test]
fn zero_sized_values_and_empty_buffers_are_supported() {
    for len in 0..=4 {
        let mut storage = [(); 4];
        let mut reserved = InOutBufReserved::from_mut_slice(&mut storage, len).unwrap();
        let (mut body, suffix) = reserved.split_reserved();
        body.get_out().fill(());
        suffix.fill(());
        assert_eq!(body.get_in().len(), len);
        assert_eq!(suffix.len(), 4 - len);
    }
    let mut storage: [u8; 0] = [];
    let mut reserved = InOutBufReserved::from_mut_slice(&mut storage, 0).unwrap();
    let (body, suffix) = reserved.split_reserved();
    assert!(body.get_in().is_empty());
    assert!(suffix.is_empty());
}
