use image::DynamicImage;

#[derive(Debug)]
pub struct ExtractedImage {
    pub image: DynamicImage,
    pub label: ImageLabel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Image,
    Pdf,
    Office,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageLabel {
    Single,
    Page { index: usize },
}
