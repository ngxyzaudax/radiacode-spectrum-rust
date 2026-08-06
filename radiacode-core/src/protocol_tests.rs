#[cfg(test)]
mod tests {
    use crate::command::Command;
    use crate::protocol::{
        build_request, framed_request_header, request_header, response_matches_request,
        ResponseAssembler, Sequence,
    };

    #[test]
    fn framed_request_header_reads_command_bytes() {
        let request = build_request(Command::GetVersion, 0x81, &[]);
        assert_eq!(
            framed_request_header(&request).unwrap(),
            request_header(Command::GetVersion, 0x81)
        );
    }

    #[test]
    fn response_match_requires_echoed_header() {
        let header = request_header(Command::GetVersion, 0x81);
        let mut payload = header.to_vec();
        payload.extend_from_slice(&[1, 2, 3]);
        assert!(response_matches_request(&payload, header));
        assert!(!response_matches_request(&[0, 0, 0, 0], header));
    }

    #[test]
    fn sequence_wraps_within_32() {
        let mut sequence = Sequence::default();
        assert_eq!(sequence.next(), 0x80);
        for _ in 0..31 {
            let _ = sequence.next();
        }
        assert_eq!(sequence.next(), 0x80);
    }

    #[test]
    fn build_request_frames_length_and_header() {
        let request = build_request(Command::GetVersion, 0x81, &[]);
        assert_eq!(&request[..4], &4u32.to_le_bytes());
        assert_eq!(&request[4..], &request_header(Command::GetVersion, 0x81));
    }

    #[test]
    fn assembler_reassembles_chunked_payload() {
        let mut assembler = ResponseAssembler::default();
        let payload = vec![1, 2, 3, 4, 5];
        let mut first = (payload.len() as i32).to_le_bytes().to_vec();
        first.extend_from_slice(&payload[..2]);
        assert!(assembler.push(&first).unwrap().is_none());
        let complete = assembler.push(&payload[2..]).unwrap().unwrap();
        assert_eq!(complete, payload);
    }
}
