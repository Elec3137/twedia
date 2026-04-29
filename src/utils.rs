use iced::Size;

pub trait BoolToggleExt {
    fn toggle(&mut self);
}

impl BoolToggleExt for bool {
    fn toggle(&mut self) {
        *self = !*self;
    }
}

pub trait SizeRatioExt<T> {
    fn get_aspect_ratio(&self) -> T;
}

impl<T: std::ops::Div<Output = T> + Copy> SizeRatioExt<T> for Size<T> {
    fn get_aspect_ratio(&self) -> T {
        self.width / self.height
    }
}
