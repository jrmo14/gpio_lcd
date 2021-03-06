pub enum Icon {
    MAIL,
    BELL,
    FILLEDBOX,
    EMPTYBOX,
    MUSIC,
    PLAY,
    PAUSE,
}

impl Icon {
    pub fn char_data(&self) -> [u8; 8] {
        match *self {
            Icon::MAIL => [0x00, 0x00, 0x00, 0x1F, 0x1B, 0x15, 0x11, 0x1F],
            Icon::BELL => [0x00, 0x04, 0x0A, 0x0A, 0x11, 0x11, 0x1F, 0x04],
            Icon::FILLEDBOX => [0x00, 0x1F, 0x11, 0x15, 0x11, 0x1F, 0x00, 0x00],
            Icon::EMPTYBOX => [0x00, 0x1F, 0x11, 0x11, 0x11, 0x1F, 0x00, 0x00],
            Icon::MUSIC => [0x00, 0x00, 0x00, 0x0F, 0x09, 0x09, 0x09, 0x12],
            Icon::PLAY => [0x00, 0x02, 0x06, 0x0E, 0x1E, 0x0E, 0x06, 0x02],
            Icon::PAUSE => [0x1B, 0x1B, 0x1B, 0x1B, 0x1B, 0x1B, 0x1B, 0x1B],
        }
    }

    pub fn index(&self) -> u8 {
        match *self {
            Icon::MAIL => 0,
            Icon::BELL => 1,
            Icon::FILLEDBOX => 2,
            Icon::EMPTYBOX => 3,
            Icon::MUSIC => 4,
            Icon::PLAY => 5,
            Icon::PAUSE => 6,
        }
    }
}
