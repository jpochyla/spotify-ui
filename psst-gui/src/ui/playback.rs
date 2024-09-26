use std::time::Duration;

use druid::{
    kurbo::{Affine, BezPath},
    widget::{CrossAxisAlignment, Either, Flex, Label, LineBreaking, Spinner, ViewSwitcher},
    BoxConstraints, Cursor, Data, Env, Event, EventCtx, LayoutCtx, LensExt, LifeCycle,
    LifeCycleCtx, MouseButton, PaintCtx, Point, Rect, RenderContext, Size, UpdateCtx, Widget,
    WidgetExt, WidgetPod,
};
use itertools::Itertools;

use crate::{
    cmd::{self},
    controller::PlaybackController,
    data::{
        AppState, AudioAnalysis, Episode, NowPlaying, Playable, PlayableMatcher, Playback,
        PlaybackOrigin, PlaybackState, QueueBehavior, ShowLink, Track,
    },
    widget::{
        icons::{self, SvgIcon},
        Empty, Maybe, MyWidgetExt, RemoteImage,
    },
};

use super::{episode, library, theme, track, utils};

pub fn panel_widget() -> impl Widget<AppState> {
    let seek_bar = Maybe::or_empty(SeekBar::new).lens(Playback::now_playing);
    let item_info = Maybe::or_empty(playing_item_widget).lens(Playback::now_playing);
    let controls = Either::new(
        |playback, _| playback.now_playing.is_some(),
        player_widget(),
        Empty,
    );
    Flex::column()
        .with_child(seek_bar)
        .with_child(BarLayout::new(item_info, controls))
        .lens(AppState::playback)
        .controller(PlaybackController::new())
}

fn playing_item_widget() -> impl Widget<NowPlaying> {
    let cover_art = cover_widget(theme::grid(8.0));

    let name = PlayableMatcher::new()
        .track(
            Label::raw()
                .with_line_break_mode(LineBreaking::Clip)
                .with_font(theme::UI_FONT_MEDIUM)
                .lens(Track::name.in_arc()),
        )
        .episode(
            Label::raw()
                .with_line_break_mode(LineBreaking::Clip)
                .with_font(theme::UI_FONT_MEDIUM)
                .lens(Episode::name.in_arc()),
        )
        .lens(NowPlaying::item);

    let detail = PlayableMatcher::new()
        .track(
            Label::raw()
                .with_line_break_mode(LineBreaking::Clip)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(Track::lens_artist_name().in_arc()),
        )
        .episode(
            Label::raw()
                .with_line_break_mode(LineBreaking::Clip)
                .with_text_size(theme::TEXT_SIZE_SMALL)
                .lens(Episode::show.in_arc().then(ShowLink::name)),
        )
        .lens(NowPlaying::item);

    let origin = ViewSwitcher::new(
        |origin: &PlaybackOrigin, _| origin.clone(),
        |origin, _, _| {
            Flex::row()
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .with_flex_child(
                    Label::dynamic(|origin: &PlaybackOrigin, _| origin.to_string())
                        .with_line_break_mode(LineBreaking::Clip)
                        .with_text_size(theme::TEXT_SIZE_SMALL),
                    1.0,
                )
                .with_spacer(theme::grid(0.25))
                .with_child(playback_origin_icon(origin).scale(theme::ICON_SIZE_SMALL))
                .boxed()
        },
    )
    .lens(NowPlaying::origin);

    Flex::row()
        .with_child(cover_art)
        .with_flex_child(
            Flex::row().with_spacer(theme::grid(2.0)).with_flex_child(
                Flex::column()
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .with_child(name)
                    .with_spacer(2.0)
                    .with_child(detail)
                    .with_spacer(2.0)
                    .with_child(origin)
                    .on_click(|ctx, now_playing, _| {
                        if now_playing.origin.to_string() != PlaybackOrigin::Queue.to_string() {
                            ctx.submit_command(cmd::NAVIGATE.with(now_playing.origin.to_nav()));
                        }
                    })
                    .context_menu(|now_playing| match &now_playing.item {
                        Playable::Track(track) => track::track_menu(
                            track,
                            &now_playing.library,
                            &now_playing.origin,
                            usize::MAX,
                        ),
                        Playable::Episode(episode) => {
                            episode::episode_menu(episode, &now_playing.library)
                        }
                    }),
                1.0,
            ),
            1.0,
        )
        .with_child(ViewSwitcher::new(
            |now_playing: &NowPlaying, _| {
                now_playing.item.track().is_some() && now_playing.library.saved_tracks.is_resolved()
            },
            |selector, _data, _env| match selector {
                true => {
                    // View is only show if now_playing's track isn't none
                    ViewSwitcher::new(
                        |now_playing: &NowPlaying, _| {
                            now_playing
                                .library
                                .contains_track(now_playing.item.track().unwrap())
                        },
                        |selector: &bool, _, _| {
                            match selector {
                                true => &icons::CIRCLE_CHECK,
                                false => &icons::CIRCLE_PLUS,
                            }
                            .scale(theme::ICON_SIZE_SMALL)
                            .boxed()
                        },
                    )
                    .on_left_click(|ctx, _, now_playing, _| {
                        let track = now_playing.item.track().unwrap();
                        if now_playing.library.contains_track(track) {
                            ctx.submit_command(library::UNSAVE_TRACK.with(track.id))
                        } else {
                            ctx.submit_command(library::SAVE_TRACK.with(track.clone()))
                        }
                    })
                    .padding(theme::grid(1.0))
                    .boxed()
                }
                false => Box::new(Flex::column()),
            },
        ))
        .padding(theme::grid(1.0))
        .link()
}

fn cover_widget(size: f64) -> impl Widget<NowPlaying> {
    RemoteImage::new(utils::placeholder_widget(), move |np: &NowPlaying, _| {
        np.cover_image_url(size, size).map(|url| url.into())
    })
    .fix_size(size, size)
    .clip(Size::new(size, size).to_rounded_rect(4.0))
}

fn playback_origin_icon(origin: &PlaybackOrigin) -> &'static SvgIcon {
    match origin {
        PlaybackOrigin::Home => &icons::HOME,
        PlaybackOrigin::Library => &icons::HEART,
        PlaybackOrigin::Queue => &icons::PLAYLIST,
        PlaybackOrigin::Album { .. } => &icons::ALBUM,
        PlaybackOrigin::Artist { .. } => &icons::ARTIST,
        PlaybackOrigin::Playlist { .. } => &icons::PLAYLIST,
        PlaybackOrigin::Show { .. } => &icons::PODCAST,
        PlaybackOrigin::Search { .. } => &icons::SEARCH,
        PlaybackOrigin::Recommendations { .. } => &icons::SEARCH,
    }
}

fn player_widget() -> impl Widget<Playback> {
    Flex::row()
        .with_child(
            small_button_widget(&icons::SKIP_BACK)
                .on_left_click(|ctx, _, _, _| ctx.submit_command(cmd::PLAY_PREVIOUS)),
        )
        .with_default_spacer()
        .with_child(player_play_pause_widget())
        .with_default_spacer()
        .with_child(
            small_button_widget(&icons::SKIP_FORWARD)
                .on_left_click(|ctx, _, _, _| ctx.submit_command(cmd::PLAY_NEXT)),
        )
        .with_default_spacer()
        .with_child(queue_behavior_widget())
        .with_default_spacer()
        .with_child(Maybe::or_empty(durations_widget).lens(Playback::now_playing))
        .padding(theme::grid(2.0))
}

fn player_play_pause_widget() -> impl Widget<Playback> {
    ViewSwitcher::new(
        |playback: &Playback, _| playback.state,
        |state, _, _| match state {
            PlaybackState::Loading => Spinner::new()
                .with_color(theme::GREY_400)
                .fix_size(theme::grid(3.0), theme::grid(3.0))
                .padding(theme::grid(1.0))
                .link()
                .circle()
                .border(theme::GREY_600, 1.0)
                .on_left_click(|ctx, _, _, _| ctx.submit_command(cmd::PLAY_STOP))
                .boxed(),
            PlaybackState::Playing => icons::PAUSE
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .link()
                .circle()
                .border(theme::GREY_500, 1.0)
                .on_left_click(|ctx, _, _, _| ctx.submit_command(cmd::PLAY_PAUSE))
                .boxed(),
            PlaybackState::Paused => icons::PLAY
                .scale((theme::grid(3.0), theme::grid(3.0)))
                .padding(theme::grid(1.0))
                .link()
                .circle()
                .border(theme::GREY_500, 1.0)
                .on_left_click(|ctx, _, _, _| ctx.submit_command(cmd::PLAY_RESUME))
                .boxed(),
            PlaybackState::Stopped => Empty.boxed(),
        },
    )
}

fn queue_behavior_widget() -> impl Widget<Playback> {
    ViewSwitcher::new(
        |playback: &Playback, _| playback.queue_behavior,
        |behavior, _, _| {
            faded_button_widget(queue_behavior_icon(behavior))
                .on_left_click(|ctx, _, playback: &mut Playback, _| {
                    ctx.submit_command(
                        cmd::PLAY_QUEUE_BEHAVIOR
                            .with(cycle_queue_behavior(&playback.queue_behavior)),
                    );
                })
                .boxed()
        },
    )
}

fn cycle_queue_behavior(qb: &QueueBehavior) -> QueueBehavior {
    match qb {
        QueueBehavior::Sequential => QueueBehavior::Random,
        QueueBehavior::Random => QueueBehavior::LoopTrack,
        QueueBehavior::LoopTrack => QueueBehavior::LoopAll,
        QueueBehavior::LoopAll => QueueBehavior::Sequential,
    }
}

fn queue_behavior_icon(qb: &QueueBehavior) -> &'static SvgIcon {
    match qb {
        QueueBehavior::Sequential => &icons::PLAY_SEQUENTIAL,
        QueueBehavior::Random => &icons::PLAY_SHUFFLE,
        QueueBehavior::LoopTrack => &icons::PLAY_LOOP_TRACK,
        QueueBehavior::LoopAll => &icons::PLAY_LOOP_ALL,
    }
}

fn small_button_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(2.0), theme::grid(2.0)))
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
}

fn faded_button_widget<T: Data>(svg: &SvgIcon) -> impl Widget<T> {
    svg.scale((theme::grid(2.0), theme::grid(2.0)))
        .with_color(theme::PLACEHOLDER_COLOR)
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
}

fn durations_widget() -> impl Widget<NowPlaying> {
    Label::dynamic(|now_playing: &NowPlaying, _| {
        format!(
            "{} / {}",
            utils::as_minutes_and_seconds(now_playing.progress),
            utils::as_minutes_and_seconds(now_playing.item.duration())
        )
    })
    .with_text_size(theme::TEXT_SIZE_SMALL)
    .with_text_color(theme::PLACEHOLDER_COLOR)
    .fix_width(theme::grid(8.0))
}

struct BarLayout<T, I, P> {
    item: WidgetPod<T, I>,
    player: WidgetPod<T, P>,
}

impl<T, I, P> BarLayout<T, I, P>
where
    T: Data,
    I: Widget<T>,
    P: Widget<T>,
{
    fn new(item: I, player: P) -> Self {
        Self {
            item: WidgetPod::new(item),
            player: WidgetPod::new(player),
        }
    }
}

impl<T, I, P> Widget<T> for BarLayout<T, I, P>
where
    T: Data,
    I: Widget<T>,
    P: Widget<T>,
{
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut T, env: &Env) {
        self.item.event(ctx, event, data, env);
        self.player.event(ctx, event, data, env);
    }

    fn lifecycle(&mut self, ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        self.item.lifecycle(ctx, event, data, env);
        self.player.lifecycle(ctx, event, data, env);
    }

    fn update(&mut self, ctx: &mut UpdateCtx, _old_data: &T, data: &T, env: &Env) {
        self.item.update(ctx, data, env);
        self.player.update(ctx, data, env);
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, bc: &BoxConstraints, data: &T, env: &Env) -> Size {
        let max = bc.max();

        const PLAYER_OPTICAL_CENTER: f64 = 60.0 + theme::GRID * 2.0;

        // Layout the player with loose constraints.
        let player = self.player.layout(ctx, &bc.loosen(), data, env);
        let player_centered = max.width > player.width * 2.25;

        // Layout the item to the available space.
        let item_max = if player_centered {
            Size::new(max.width * 0.5 - PLAYER_OPTICAL_CENTER, max.height)
        } else {
            Size::new(max.width - player.width, max.height)
        };
        let item = self
            .item
            .layout(ctx, &BoxConstraints::new(Size::ZERO, item_max), data, env);

        let total = Size::new(max.width, player.height.max(item.height));

        // Put the item to the top left.
        self.item.set_origin(ctx, Point::ORIGIN);

        // Put the player either to the center or to the right.
        let player_pos = if player_centered {
            Point::new(
                total.width * 0.5 - PLAYER_OPTICAL_CENTER,
                total.height * 0.5 - player.height * 0.5,
            )
        } else {
            Point::new(
                total.width - player.width,
                total.height * 0.5 - player.height * 0.5,
            )
        };
        self.player.set_origin(ctx, player_pos);

        total
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &T, env: &Env) {
        self.item.paint(ctx, data, env);
        self.player.paint(ctx, data, env);
    }
}

struct SeekBar {
    loudness_path: BezPath,
}

impl SeekBar {
    fn new() -> Self {
        Self {
            loudness_path: BezPath::new(),
        }
    }
}

impl Widget<NowPlaying> for SeekBar {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, _data: &mut NowPlaying, _env: &Env) {
        match event {
            Event::MouseMove(_) => {
                ctx.set_cursor(&Cursor::Pointer);
            }
            Event::MouseDown(mouse) => {
                if mouse.button == MouseButton::Left {
                    ctx.set_active(true);
                }
            }
            Event::MouseUp(mouse) => {
                if ctx.is_active() && mouse.button == MouseButton::Left {
                    if ctx.is_hot() {
                        let fraction = mouse.pos.x / ctx.size().width;
                        ctx.submit_command(cmd::PLAY_SEEK.with(fraction));
                    }
                    ctx.set_active(false);
                }
            }
            _ => {}
        }
    }

    fn lifecycle(
        &mut self,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &NowPlaying,
        _env: &Env,
    ) {
        match &event {
            LifeCycle::Size(_bounds) => {
                // self.loudness_path = compute_loudness_path(bounds, &data);
            }
            LifeCycle::HotChanged(_) => {
                ctx.request_paint();
            }
            _ => {}
        }
    }

    fn update(
        &mut self,
        ctx: &mut UpdateCtx,
        old_data: &NowPlaying,
        data: &NowPlaying,
        _env: &Env,
    ) {
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &NowPlaying,
        _env: &Env,
    ) -> Size {
        Size::new(bc.max().width, theme::grid(1.0))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &NowPlaying, env: &Env) {
        if self.loudness_path.is_empty() {
            paint_progress_bar(ctx, data, env)
        } else {
            paint_audio_analysis(ctx, data, &self.loudness_path, env)
        }
    }
}

fn _compute_loudness_path_from_analysis(
    bounds: &Size,
    total_duration: &Duration,
    analysis: &AudioAnalysis,
) -> BezPath {
    let (loudness_min, loudness_max) = analysis
        .segments
        .iter()
        .map(|s| s.loudness_max)
        .minmax()
        .into_option()
        .unwrap_or((0.0, 0.0));
    let total_loudness = loudness_max - loudness_min;

    let mut path = BezPath::new();

    // We start in the middle of the vertical space and first draw the upper half of
    // the curve, then take what we have drawn, flip the y-axis and append it
    // underneath.
    let origin_y = bounds.height / 2.0;

    // Start at the origin.
    path.move_to((0.0, origin_y));

    // Because the size of the seekbar is quite small, but the number of the
    // segments can be large, we down-sample the loudness spectrum in a very
    // primitive way and only add a vertex after crossing `WIDTH_PRECISION` of
    // pixels horizontally.
    const WIDTH_PRECISION: f64 = 2.0;
    let mut last_width = 0.0;

    for seg in &analysis.segments {
        let time = seg.interval.start.as_secs_f64() + seg.loudness_max_time;
        let tfrac = time / total_duration.as_secs_f64();
        let width = bounds.width * tfrac;

        let loud = seg.loudness_max - loudness_min;
        let lfrac = loud / total_loudness;
        let height = bounds.height * lfrac;

        if width - last_width >= WIDTH_PRECISION {
            // Down-scale the height, because we will be drawing also the inverted half.
            path.line_to((width, origin_y - height / 2.0));

            // Save the X-coordinate of this vertex.
            last_width = width;
        }
    }

    // Land back at the vertical origin.
    path.line_to((bounds.width, origin_y));

    // Flip the y-axis, translate just under the origin, and append.
    let mut inverted_path = path.clone();
    let inversion_tx = Affine::FLIP_Y * Affine::translate((0.0, -bounds.height));
    inverted_path.apply_affine(inversion_tx);
    path.extend(inverted_path);

    path
}

fn paint_audio_analysis(ctx: &mut PaintCtx, data: &NowPlaying, path: &BezPath, env: &Env) {
    let bounds = ctx.size();

    let elapsed_time = data.progress.as_secs_f64();
    let total_time = data.item.duration().as_secs_f64();
    let elapsed_frac = elapsed_time / total_time;
    let elapsed_width = bounds.width * elapsed_frac;
    let elapsed = Size::new(elapsed_width, bounds.height).to_rect();

    let (elapsed_color, remaining_color) = if ctx.is_hot() {
        (env.get(theme::GREY_200), env.get(theme::GREY_500))
    } else {
        (env.get(theme::GREY_300), env.get(theme::GREY_600))
    };

    ctx.with_save(|ctx| {
        ctx.fill(path, &remaining_color);
        ctx.clip(elapsed);
        ctx.fill(path, &elapsed_color);
    });
}

fn paint_progress_bar(ctx: &mut PaintCtx, data: &NowPlaying, env: &Env) {
    let elapsed_time = data.progress.as_secs_f64();
    let total_time = data.item.duration().as_secs_f64();

    let (elapsed_color, remaining_color) = if ctx.is_hot() {
        (env.get(theme::GREY_200), env.get(theme::GREY_500))
    } else {
        (env.get(theme::GREY_300), env.get(theme::GREY_600))
    };
    let bounds = ctx.size();

    let elapsed_frac = elapsed_time / total_time;
    let elapsed_width = bounds.width * elapsed_frac;
    let remaining_width = bounds.width - elapsed_width;
    let elapsed = Size::new(elapsed_width, bounds.height).round();
    let remaining = Size::new(remaining_width, bounds.height).round();

    ctx.fill(
        Rect::from_origin_size(Point::ORIGIN, elapsed),
        &elapsed_color,
    );
    ctx.fill(
        Rect::from_origin_size(Point::new(elapsed.width, 0.0), remaining),
        &remaining_color,
    );
}
