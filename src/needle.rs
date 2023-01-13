use iced::widget::canvas;
use iced::widget::canvas::{stroke, Cache, Cursor, Geometry, LineCap, Path, Stroke};
use iced::{Color, Point, Rectangle, Theme, Vector};

#[derive(Debug)]
pub struct Needle {
    rotation: u32,
    needle: Cache,
}
impl Needle {
    pub fn new(rotation: u32) -> Self {
        Self {
            rotation,
            needle: Cache::default(),
        }
    }
}

impl<Message> canvas::Program<Message> for Needle {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let needle = self.needle.draw(bounds.size(), |frame| {
            let center = frame.center();
            let radius = frame.width().min(frame.height()) / 2.0;

            frame.fill(
                &Path::circle(center, radius),
                Color::from_rgb8(0x66, 0x34, 0x99),
            );

            let long_hand = Path::line(Point::ORIGIN, Point::new(0.0, -0.9 * radius));

            let wide_stroke = || -> Stroke {
                Stroke {
                    width: radius / 5.0,
                    style: stroke::Style::Solid(Color::WHITE),
                    line_cap: LineCap::Round,
                    ..Stroke::default()
                }
            };

            frame.translate(Vector::new(center.x, center.y));
            frame.rotate((self.rotation as f32).to_radians());
            frame.stroke(&long_hand, wide_stroke());
        });

        vec![needle]
    }
}
