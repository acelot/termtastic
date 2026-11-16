use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::Widget,
};

#[derive(Clone)]
pub struct ColumnsWidget<T, const N: usize> {
    pub cols: [Option<T>; N],
}

impl<T, const N: usize> ColumnsWidget<T, N> {
    pub fn new(cols: [Option<T>; N]) -> Self {
        Self { cols }
    }
}

impl<T: Widget, const N: usize> Widget for ColumnsWidget<T, N> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if N == 0 {
            return;
        }

        let columns = Layout::horizontal([Constraint::Fill(1); N]).split(area);

        for (col, area) in self.cols.into_iter().zip(columns.iter()) {
            if let Some(widget) = col {
                widget.render(*area, buf);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Style;

    struct Mark(&'static str);

    impl Widget for Mark {
        fn render(self, area: Rect, buf: &mut Buffer) {
            buf.set_string(area.x, area.y, self.0, Style::default());
        }
    }

    fn render<T: Widget, const N: usize>(widget: ColumnsWidget<T, N>, width: u16) -> String {
        let area = Rect::new(0, 0, width, 1);
        let mut buf = Buffer::empty(area);
        widget.render(area, &mut buf);
        (0..width).map(|x| buf[(x, 0)].symbol().to_owned()).collect()
    }

    #[test]
    fn splits_area_into_equal_columns() {
        let line = render(
            ColumnsWidget::new([Some(Mark("a")), None, Some(Mark("c")), Some(Mark("d"))]),
            12,
        );
        assert_eq!(line, "a     c  d  ");
    }

    #[test]
    fn handles_single_column() {
        let line = render(ColumnsWidget::new([Some(Mark("a"))]), 4);
        assert_eq!(line, "a   ");
    }

    #[test]
    fn handles_no_columns() {
        let line = render(ColumnsWidget::<Mark, 0>::new([]), 4);
        assert_eq!(line, "    ");
    }
}
