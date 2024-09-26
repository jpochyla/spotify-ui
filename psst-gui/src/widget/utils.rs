use druid::{
    kurbo::{Line, Shape},
    widget::{prelude::*, Axis, BackgroundBrush, Painter},
    Color, Data, KeyOrValue,
};

pub struct FadeOut<W> {
    inner: W,
    axis: Axis,
    limit: KeyOrValue<f64>,
    color: KeyOrValue<Color>,
    over_limit: bool,
}

impl<W> FadeOut<W> {
    // const FADE_LENGTH_MAX: f64 = 32.0;

    pub fn new(inner: W, axis: Axis, limit: KeyOrValue<f64>) -> Self {
        Self {
            inner,
            axis,
            limit,
            color: Color::BLACK.into(),
            over_limit: false,
        }
    }

    pub fn bottom(inner: W, height: impl Into<KeyOrValue<f64>>) -> Self {
        Self::new(inner, Axis::Vertical, height.into())
    }

    pub fn with_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.color = color.into();
        self
    }
}

impl<T, W> Widget<T> for FadeOut<W>
where
    T: Data,
    W: Widget<T>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(
            ctx,
            &BoxConstraints::new(
                bc.min(),
                match self.axis {
                    // Unbounded with.
                    Axis::Horizontal => Size::new(f64::INFINITY, bc.max().height),
                    // Unbounded height.
                    Axis::Vertical => Size::new(bc.max().width, f64::INFINITY),
                },
            ),
            data,
            env,
        );
        let limit = self.limit.resolve(env);
        self.over_limit = self.axis.major(size) > limit;
        if self.over_limit {
            Size::from(self.axis.pack(limit, self.axis.minor(size)))
        } else {
            size
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        let size = ctx.size();
        let rect = size.to_rect();
        if self.over_limit {
            // Clip and paint inner widget.
            ctx.with_save(|ctx| {
                ctx.clip(rect);
                self.inner.paint(ctx, data, env);
            });
            // Paint the gradient.
            // let gradient_rect = match self.axis {
            //     Axis::Horizontal => Rect::new(
            //         0.0f64.max(size.width - Self::FADE_LENGTH_MAX),
            //         0.0,
            //         size.width,
            //         size.height,
            //     ),
            //     Axis::Vertical => Rect::new(
            //         0.0,
            //         0.0f64.max(size.height - Self::FADE_LENGTH_MAX),
            //         size.width,
            //         size.height,
            //     ),
            // };
            // let (start, end) = match self.axis {
            //     Axis::Horizontal => (UnitPoint::LEFT, UnitPoint::RIGHT),
            //     Axis::Vertical => (UnitPoint::TOP, UnitPoint::BOTTOM),
            // };
            // ctx.fill(
            //     &gradient_rect,
            //     &LinearGradient::new(
            //         start,
            //         end,
            //         (
            //             self.color.resolve(env).with_alpha(0.0),
            //             self.color.resolve(env),
            //         ),
            //     ),
            // );
        } else {
            self.inner.paint(ctx, data, env);
        }
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}

pub struct Clip<S, W> {
    shape: S,
    inner: W,
}

impl<S, W> Clip<S, W> {
    pub fn new(shape: S, inner: W) -> Self {
        Self { shape, inner }
    }
}

impl<T: Data, S: Shape, W: Widget<T>> Widget<T> for Clip<S, W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let size = self.inner.layout(ctx, bc, data, env);
        let bbox = self.shape.bounding_box().size();
        Size::new(size.width.min(bbox.width), size.height.min(bbox.height))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        ctx.with_save(|ctx| {
            ctx.clip(&self.shape);
            self.inner.paint(ctx, data, env);
        });
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}

pub enum Border {
    Top,
    Bottom,
    // Todo implement this so then we can have borders on the sides.
    // This is needed as the split widget had the border.
    Left,
    //Right,
}

impl Border {
    pub fn with_color<T: Data>(
        self,
        color: impl Into<KeyOrValue<Color>>,
    ) -> impl Into<BackgroundBrush<T>> {
        let color = color.into();

        Painter::new(move |ctx, _, env| {
            let h = 1.0;
            let w1 = match self {
                Self::Top => 0.0,
                Self::Bottom => 0.0,
                Self::Left => h / 2.0,
                //Self::Right => ctx.size().width - h / 2.0,
            };
            let w2 = match self {
                Self::Top => ctx.size().width,
                Self::Bottom => ctx.size().width,
                Self::Left => h / 2.0,
                //Self::Right => ctx.size().width - h / 2.0,
            };
            let y1 = match self {
                Self::Top => h / 2.0,
                Self::Bottom => ctx.size().height - h / 2.0,
                Self::Left => ctx.size().height - h / 2.0,
                //Self::Right => ctx.size().height - h / 2.0,
            };
            let y2 = match self {
                Self::Top => h / 2.0,
                Self::Bottom => ctx.size().height - h / 2.0,
                Self::Left => h / 2.0,
                //Self::Right => h / 2.0,
            };
            let line = Line::new((w1, y1), (w2, y2));
            ctx.stroke(line, &color.resolve(env), h);
        })
    }
}

pub struct Logger<W> {
    inner: W,
    label: &'static str,
    event: bool,
    lifecycle: bool,
    update: bool,
    layout: bool,
    paint: bool,
}

#[allow(dead_code)]
impl<W> Logger<W> {
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            label: "logger",
            event: false,
            lifecycle: false,
            update: false,
            layout: false,
            paint: false,
        }
    }

    pub fn with_label(mut self, title: &'static str) -> Self {
        self.label = title;
        self
    }

    pub fn with_event(mut self) -> Self {
        self.event = true;
        self
    }

    pub fn with_lifecycle(mut self) -> Self {
        self.lifecycle = true;
        self
    }

    pub fn with_update(mut self) -> Self {
        self.update = true;
        self
    }

    pub fn with_layout(mut self) -> Self {
        self.layout = true;
        self
    }

    pub fn with_paint(mut self) -> Self {
        self.paint = true;
        self
    }
}

impl<T: Data, W: Widget<T>> Widget<T> for Logger<W> {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        if self.event {
            log::info!("{:?} event: {:?}", self.label, event);
        }
        self.inner.event(ctx, event, data, env)
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if self.lifecycle {
            log::info!("{:?} lifecycle: {:?}", self.label, event);
        }
        self.inner.lifecycle(ctx, event, data, env)
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if self.update {
            log::info!("{:?} update", self.label);
        }
        self.inner.update(ctx, old_data, data, env)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        if self.layout {
            log::info!("{:?} layout", self.label);
        }
        self.inner.layout(ctx, bc, data, env)
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        if self.paint {
            log::info!("{:?} paint", self.label);
        }
        self.inner.paint(ctx, data, env)
    }

    fn id(&self) -> Option<WidgetId> {
        self.inner.id()
    }
}
