mod frames;

pub fn parse_frame(data: &[u8]) -> Result<frames::FrameType, frames::FrameError> {
    frames::parse_frame(data)
}
