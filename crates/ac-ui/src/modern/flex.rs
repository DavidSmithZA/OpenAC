//! Flex layout for hand-built windows: the plugin markup's flexbox
//! ([`ac_plugin_ui::flex`], pure and markup-free) laid over elements of a
//! [`UiRoot`]. A modern window lists its pieces as flex nodes, the leaves
//! tied to the elements that show them and measured by what they hold;
//! [`ElementFlex::arrange`] then sizes and places those elements inside
//! their container.

use ac_plugin_ui::flex::{FlexMeasurement, FlexNode, FlexRect, FlexSize, FlexTree};

use crate::font::UiFont;
use crate::root::UiRoot;
use crate::tree::ElementId;

/// What a leaf holds, measured: the size it would like and the least it can
/// take.
pub trait Measure {
    /// The leaf's measurement.
    fn measure(&self) -> FlexMeasurement;
}

impl Measure for FlexMeasurement {
    fn measure(&self) -> FlexMeasurement {
        *self
    }
}

/// One line of text in a font, with room either side: it would like its
/// width and can be cut down to `minimum_width` (an ellipsis), never
/// shorter in height than a line.
#[derive(Debug, Clone, Copy)]
pub struct TextLine<'a> {
    /// The font.
    pub font: &'a UiFont,
    /// The text.
    pub text: &'a str,
    /// Room left and right of the text, units.
    pub padding_x: f32,
    /// Room above and below the line, units.
    pub padding_y: f32,
    /// The narrowest the text may be cut to (0: as narrow as the padding).
    pub minimum_width: f32,
}

impl Measure for TextLine<'_> {
    fn measure(&self) -> FlexMeasurement {
        let width = self.font.measure_width(self.text) + self.padding_x * 2.0;
        let height = self.font.line_height + self.padding_y * 2.0;
        let least = (self.minimum_width + self.padding_x * 2.0).min(width);
        FlexMeasurement {
            preferred: FlexSize { width, height },
            minimum: FlexSize {
                width: least,
                height,
            },
        }
    }
}

/// A fixed size: an icon, a slot, a gap of exact width.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Fixed {
    /// Width, units.
    pub width: f32,
    /// Height, units.
    pub height: f32,
}

impl Measure for Fixed {
    fn measure(&self) -> FlexMeasurement {
        let size = FlexSize {
            width: self.width,
            height: self.height,
        };
        FlexMeasurement {
            preferred: size,
            minimum: size,
        }
    }
}

/// A flex tree whose leaves are elements.
#[derive(Debug, Clone)]
pub struct ElementFlex {
    tree: FlexTree,
    root: usize,
    /// Each leaf's node and the element it places.
    elements: Vec<(usize, ElementId)>,
}

impl ElementFlex {
    /// A layout whose outermost container is `root` (its direction, gap,
    /// padding, alignment).
    #[must_use]
    pub fn new(root: FlexNode) -> Self {
        let mut tree = FlexTree::default();
        let root = tree.add(root);
        Self {
            tree,
            root,
            elements: Vec::new(),
        }
    }

    /// The outermost container's node number.
    #[must_use]
    pub fn root(&self) -> usize {
        self.root
    }

    /// Adds a container (no element of its own) under `parent`; its node
    /// number.
    pub fn container(&mut self, parent: usize, node: FlexNode) -> usize {
        self.tree.add_child(parent, node)
    }

    /// Adds a leaf under `parent` that places `element`, sized by what it
    /// holds (`content`) and `node`'s own rules (grow, shrink, fixed or
    /// limited sizes, alignment); its node number.
    pub fn element(
        &mut self,
        parent: usize,
        element: ElementId,
        content: &dyn Measure,
        mut node: FlexNode,
    ) -> usize {
        node.measure = Some(content.measure());
        let index = self.tree.add_child(parent, node);
        self.elements.push((index, element));
        index
    }

    /// The whole layout's measurement: the size it would like and the least
    /// it can take.
    pub fn measure(&mut self) -> FlexMeasurement {
        self.tree.measure(self.root)
    }

    /// Lays the tree out in a container `width` by `height` and moves each
    /// element to its place, relative to the container (the elements are
    /// its children). An element that is gone is skipped.
    pub fn arrange(&mut self, ui: &mut UiRoot, width: f32, height: f32) {
        self.tree.arrange(
            self.root,
            FlexRect {
                x: 0.0,
                y: 0.0,
                width,
                height,
            },
        );
        for &(index, element) in &self.elements {
            let r = self.tree.nodes[index].rect;
            if let Some(e) = ui.element_mut(element) {
                e.left = r.x;
                e.top = r.y;
                e.width = r.width;
                e.height = r.height;
            }
        }
    }

    /// Node `index`'s place after the last arrange.
    #[must_use]
    pub fn rect(&self, index: usize) -> Option<FlexRect> {
        self.tree.nodes.get(index).map(|n| n.rect)
    }
}

#[cfg(test)]
mod tests {
    use ac_plugin_ui::flex::{FlexAlign, FlexDirection, FlexEdges};

    use super::*;
    use crate::tree::Element;
    use crate::widgets::Panel;

    fn row() -> FlexNode {
        let mut node = FlexNode::container(FlexDirection::Row);
        node.gap = 4.0;
        node.padding = FlexEdges {
            top: 2.0,
            right: 6.0,
            bottom: 2.0,
            left: 6.0,
        };
        node.align = FlexAlign::Center;
        node
    }

    #[test]
    fn leaves_take_their_measure_and_a_grower_takes_the_rest() {
        let mut ui = UiRoot::new();
        let root = ui.root();
        let icon = ui.add(root, Element::at(0.0, 0.0, 1.0, 1.0), Panel::default());
        let name = ui.add(root, Element::at(0.0, 0.0, 1.0, 1.0), Panel::default());
        let count = ui.add(root, Element::at(0.0, 0.0, 1.0, 1.0), Panel::default());
        let mut flex = ElementFlex::new(row());
        let top = flex.root();
        let square = Fixed {
            width: 16.0,
            height: 16.0,
        };
        flex.element(top, icon, &square, FlexNode::default());
        let mut grow = FlexNode::default();
        grow.grow = 1.0;
        let text = Fixed {
            width: 50.0,
            height: 12.0,
        };
        flex.element(top, name, &text, grow);
        flex.element(top, count, &text, FlexNode::default());
        let m = flex.measure();
        assert_eq!(
            m.preferred.width,
            6.0 + 16.0 + 4.0 + 50.0 + 4.0 + 50.0 + 6.0
        );
        assert_eq!(m.preferred.height, 20.0);
        flex.arrange(&mut ui, 300.0, 28.0);
        let rect = |id| {
            let e = ui.element(id).unwrap();
            (e.left, e.top, e.width, e.height)
        };
        assert_eq!(rect(icon), (6.0, 6.0, 16.0, 16.0));
        // The name grows into what the row has spare.
        assert_eq!(rect(name), (26.0, 8.0, 300.0 - 136.0 + 50.0, 12.0));
        assert_eq!(rect(count), (244.0, 8.0, 50.0, 12.0));
    }

    #[test]
    fn a_line_of_text_measures_its_width_and_a_line_high() {
        let font = crate::bundled_font::bake(16.0, crate::bundled_font::Weight::Regular)
            .unwrap()
            .create_font(crate::draw::TextureRef::Runtime(1));
        let line = TextLine {
            font: &font,
            text: "Drudge Ravener",
            padding_x: 8.0,
            padding_y: 3.0,
            minimum_width: 20.0,
        };
        let m = line.measure();
        assert_eq!(
            m.preferred.width,
            font.measure_width("Drudge Ravener") + 16.0
        );
        assert_eq!(m.preferred.height, font.line_height + 6.0);
        assert_eq!(m.minimum.width, 36.0);
    }
}
