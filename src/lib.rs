mod frames;
mod user_data;

pub fn parse_frame(data: &[u8]) -> Result<frames::FrameType, frames::FrameError> {
    let frame = frames::parse_frame(data);

    if let Ok(frames::FrameType::LongFrame{function, address, data}) = &frame {
        let user_data = user_data::parse_user_data(data);
        println!("Function: {:?}, Address: {:?}, User Data: {:?}", function, address, user_data);
    }

    frame
}
