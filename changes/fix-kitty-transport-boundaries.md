# Bound Kitty image transfer allocation

- Reject oversized image-size declarations before payload decoding and stop reserving memory for bytes that have not arrived.
- Accumulate received chunks with checked byte limits and fallible capacity growth, preserving padded chunks, first-chunk metadata, and quiet replies.
- Retire only the rejected transfer so unrelated pending images and subsequent valid uploads remain usable.
