use std::thread::{self, JoinHandle};

use druid::{
    commands,
    widget::{
        Button, Controller, CrossAxisAlignment, Flex, Label, LineBreaking, MainAxisAlignment,
        RadioGroup, TextBox, ViewSwitcher,
    },
    Data, Env, Event, EventCtx, LensExt, LifeCycle, LifeCycleCtx, Selector, Widget, WidgetExt,
};
use psst_core::connection::Credentials;

use crate::{
    cmd,
    controller::InputController,
    data::{
        AppState, AudioQuality, Authentication, Config, Preferences, PreferencesTab, Promise, Theme,
    },
    widget::{icons, Async, Border, MyWidgetExt},
};

use super::{icons::SvgIcon, theme};

pub fn account_setup_widget() -> impl Widget<AppState> {
    Flex::column()
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .with_spacer(theme::grid(2.0))
        .with_child(
            Label::new("Please insert your Spotify Premium credentials.")
                .with_font(theme::UI_FONT_MEDIUM)
                .with_line_break_mode(LineBreaking::WordWrap),
        )
        .with_spacer(theme::grid(2.0))
        .with_child(
            Label::new(
                "Psst connects only to the official servers, and does not store your password.",
            )
            .with_text_color(theme::PLACEHOLDER_COLOR)
            .with_line_break_mode(LineBreaking::WordWrap),
        )
        .with_spacer(theme::grid(6.0))
        .with_child(account_tab_widget(AccountTab::FirstSetup).expand_width())
        .padding(theme::grid(4.0))
}

pub fn preferences_widget() -> impl Widget<AppState> {
    Flex::column()
        .must_fill_main_axis(true)
        .cross_axis_alignment(CrossAxisAlignment::Fill)
        .with_child(
            tabs_widget()
                .padding(theme::grid(2.0))
                .background(theme::BACKGROUND_LIGHT),
        )
        .with_child(
            ViewSwitcher::new(
                |state: &AppState, _| state.preferences.active,
                |active, _, _| match active {
                    PreferencesTab::General => general_tab_widget().boxed(),
                    PreferencesTab::Account => {
                        account_tab_widget(AccountTab::InPreferences).boxed()
                    }
                    PreferencesTab::Cache => cache_tab_widget().boxed(),
                },
            )
            .padding(theme::grid(4.0))
            .background(Border::Top.with_color(theme::GREY_500)),
        )
        .on_update(|_, old_data, data, _| {
            // Immediately save any changes in the config.
            if !old_data.config.same(&data.config) {
                data.config.save();
            }
        })
}

fn tabs_widget() -> impl Widget<AppState> {
    Flex::row()
        .must_fill_main_axis(true)
        .main_axis_alignment(MainAxisAlignment::Center)
        .with_child(tab_link_widget(
            "General",
            &icons::PREFERENCES,
            PreferencesTab::General,
        ))
        .with_default_spacer()
        .with_child(tab_link_widget(
            "Account",
            &icons::ACCOUNT,
            PreferencesTab::Account,
        ))
        .with_default_spacer()
        .with_child(tab_link_widget(
            "Cache",
            &icons::STORAGE,
            PreferencesTab::Cache,
        ))
}

fn tab_link_widget(
    text: &'static str,
    icon: &SvgIcon,
    tab: PreferencesTab,
) -> impl Widget<AppState> {
    Flex::column()
        .with_child(icon.scale(theme::ICON_SIZE))
        .with_default_spacer()
        .with_child(Label::new(text).with_font(theme::UI_FONT_MEDIUM))
        .padding(theme::grid(1.0))
        .link()
        .rounded(theme::BUTTON_BORDER_RADIUS)
        .active(move |state: &AppState, _| tab == state.preferences.active)
        .on_click(move |_, state: &mut AppState, _| {
            state.preferences.active = tab;
        })
        .env_scope(|env, _| {
            env.set(theme::LINK_ACTIVE_COLOR, env.get(theme::BACKGROUND_DARK));
        })
}

fn general_tab_widget() -> impl Widget<AppState> {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    // Theme
    col = col
        .with_child(Label::new("Theme").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            RadioGroup::new(vec![("Light", Theme::Light), ("Dark", Theme::Dark), ("Auto", Theme::Auto)])
                .lens(AppState::config.then(Config::theme)),
        );

    col = col.with_spacer(theme::grid(3.0));

    // Audio quality
    col = col
        .with_child(Label::new("Audio quality").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            RadioGroup::new(vec![
                ("Low (96kbit)", AudioQuality::Low),
                ("Normal (160kbit)", AudioQuality::Normal),
                ("High (320kbit)", AudioQuality::High),
            ])
            .lens(AppState::config.then(Config::audio_quality)),
        );

    col
}

#[derive(Copy, Clone)]
enum AccountTab {
    FirstSetup,
    InPreferences,
}

fn account_tab_widget(tab: AccountTab) -> impl Widget<AppState> {
    let mut col = Flex::column().cross_axis_alignment(match tab {
        AccountTab::FirstSetup => CrossAxisAlignment::Center,
        AccountTab::InPreferences => CrossAxisAlignment::Start,
    });

    if matches!(tab, AccountTab::InPreferences) {
        col = col
            .with_child(Label::new("Credentials").with_font(theme::UI_FONT_MEDIUM))
            .with_spacer(theme::grid(2.0));
    }

    col = col
        .with_child(
            TextBox::new()
                .with_placeholder("Username")
                .controller(InputController::new())
                .env_scope(|env, _| env.set(theme::WIDE_WIDGET_WIDTH, theme::grid(16.0)))
                .lens(
                    AppState::preferences
                        .then(Preferences::auth)
                        .then(Authentication::username),
                ),
        )
        .with_spacer(theme::grid(1.0));

    col = col
        .with_child(
            TextBox::new()
                .with_placeholder("Password")
                .controller(InputController::new())
                .env_scope(|env, _| env.set(theme::WIDE_WIDGET_WIDTH, theme::grid(16.0)))
                .lens(
                    AppState::preferences
                        .then(Preferences::auth)
                        .then(Authentication::password),
                ),
        )
        .with_spacer(theme::grid(1.0));

    col = col
        .with_child(
            Button::new(match &tab {
                AccountTab::FirstSetup => "Log In & Continue",
                AccountTab::InPreferences => "Change Account",
            })
            .on_click(|ctx, _, _| {
                ctx.submit_command(Authenticate::REQUEST);
            }),
        )
        .with_spacer(theme::grid(1.0))
        .with_child(
            Async::new(
                || Label::new("Logging In...").with_text_size(theme::TEXT_SIZE_SMALL),
                || Label::new("Success.").with_text_size(theme::TEXT_SIZE_SMALL),
                || {
                    Label::dynamic(|err: &String, _| err.to_owned())
                        .with_text_size(theme::TEXT_SIZE_SMALL)
                        .with_text_color(theme::RED)
                },
            )
            .lens(
                AppState::preferences
                    .then(Preferences::auth)
                    .then(Authentication::result),
            ),
        );

    col = col.with_spacer(theme::grid(3.0));

    col.controller(Authenticate::new(tab))
}

struct Authenticate {
    tab: AccountTab,
    thread: Option<JoinHandle<()>>,
}

impl Authenticate {
    fn new(tab: AccountTab) -> Self {
        Self { tab, thread: None }
    }
}

impl Authenticate {
    const REQUEST: Selector = Selector::new("app.preferences.authenticate-request");
    const RESPONSE: Selector<Result<Credentials, String>> =
        Selector::new("app.preferences.authenticate-response");
}

impl<W: Widget<AppState>> Controller<AppState, W> for Authenticate {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut AppState,
        env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(Self::REQUEST) => {
                // Signal that we're authenticating.
                data.preferences.auth.result.defer_default();

                // Authenticate in another thread.
                let config = data.preferences.auth.session_config();
                let widget_id = ctx.widget_id();
                let event_sink = ctx.get_external_handle();
                let thread = thread::spawn(move || {
                    let response = Authentication::authenticate_and_get_credentials(config);
                    event_sink
                        .submit_command(Self::RESPONSE, response, widget_id)
                        .unwrap();
                });
                self.thread.replace(thread);

                ctx.set_handled();
            }
            Event::Command(cmd) if cmd.is(Self::RESPONSE) => {
                self.thread.take();

                // Store the retrieved credentials into the config.
                let result = cmd.get_unchecked(Self::RESPONSE);
                let result = result.to_owned().map(|credentials| {
                    data.config.store_credentials(credentials);
                    data.config.save();
                });
                // Signal the auth result to the preferences UI.
                data.preferences.auth.result.resolve_or_reject((), result);

                match &self.tab {
                    AccountTab::FirstSetup => {
                        // We let the `SessionController` pick up the credentials when the main
                        // window gets created. Close the account setup window and open the main
                        // one.
                        ctx.submit_command(cmd::SHOW_MAIN);
                        ctx.submit_command(commands::CLOSE_WINDOW);
                    }
                    AccountTab::InPreferences => {
                        // Drop the old connection and connect again with the new credentials.
                        ctx.submit_command(cmd::SESSION_CONNECT);
                    }
                }

                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }
}

fn cache_tab_widget() -> impl Widget<AppState> {
    let mut col = Flex::column().cross_axis_alignment(CrossAxisAlignment::Start);

    col = col
        .with_child(Label::new("Location").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(
            Label::dynamic(|_, _| {
                Config::cache_dir()
                    .map(|path| path.to_string_lossy().to_string())
                    .unwrap_or_else(|| "None".to_string())
            })
            .with_line_break_mode(LineBreaking::WordWrap),
        );

    col = col.with_spacer(theme::grid(3.0));

    col = col
        .with_child(Label::new("Size").with_font(theme::UI_FONT_MEDIUM))
        .with_spacer(theme::grid(2.0))
        .with_child(Label::dynamic(
            |preferences: &Preferences, _| match preferences.cache_size {
                Promise::Empty | Promise::Rejected { .. } => "Unknown".to_string(),
                Promise::Deferred { .. } => "Computing".to_string(),
                Promise::Resolved { val: 0, .. } => "Empty".to_string(),
                Promise::Resolved { val, .. } => {
                    format!("{:.2} MB", val as f64 / 1e6_f64)
                }
            },
        ));

    col.controller(MeasureCacheSize::new())
        .lens(AppState::preferences)
}

struct MeasureCacheSize {
    thread: Option<JoinHandle<()>>,
}

impl MeasureCacheSize {
    fn new() -> Self {
        Self { thread: None }
    }
}

impl MeasureCacheSize {
    const RESULT: Selector<Option<u64>> = Selector::new("app.preferences.measure-cache-size");
}

impl<W: Widget<Preferences>> Controller<Preferences, W> for MeasureCacheSize {
    fn event(
        &mut self,
        child: &mut W,
        ctx: &mut EventCtx,
        event: &Event,
        data: &mut Preferences,
        env: &Env,
    ) {
        match &event {
            Event::Command(cmd) if cmd.is(Self::RESULT) => {
                let result = cmd.get_unchecked(Self::RESULT).to_owned();
                data.cache_size.resolve_or_reject((), result.ok_or(()));
                self.thread.take();
                ctx.set_handled();
            }
            _ => {
                child.event(ctx, event, data, env);
            }
        }
    }

    fn lifecycle(
        &mut self,
        child: &mut W,
        ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        data: &Preferences,
        env: &Env,
    ) {
        if let LifeCycle::WidgetAdded = &event {
            let handle = thread::spawn({
                let widget_id = ctx.widget_id();
                let event_sink = ctx.get_external_handle();
                move || {
                    let size = Preferences::measure_cache_usage();
                    event_sink
                        .submit_command(Self::RESULT, size, widget_id)
                        .unwrap();
                }
            });
            self.thread.replace(handle);
        }
        child.lifecycle(ctx, event, data, env);
    }
}
