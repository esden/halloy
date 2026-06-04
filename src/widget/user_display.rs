use data::buffer::Brackets;
use data::config::buffer::{AccessLevelFormat, Dimmed};
use data::config::context_menu;
use data::config::display::nickname::Metadata;
use data::target::{Query, TargetRef};
use data::user::AccessLevel;
use data::{Config, User, metadata, preview};
use iced::{Background, Border, Color, ContentFit, Length};
use iced::alignment::Vertical;
use iced::widget::text::Wrapping;
use iced::widget::{center, container, row};
use unicode_segmentation::UnicodeSegmentation;

use super::{Element, selectable_text, text};
use crate::widget::TextExt as _;
use crate::{Theme, buffer, font, icon, theme, widget};

const AVATAR_SIZE: u16 = 18;

#[derive(Clone)]
pub struct UserDisplay<'av> {
    base: UserDisplayData<'av>,
    tooltip: Option<UserDisplayData<'av>>,
    color: Option<Color>,
}

impl <'av> UserDisplay<'av> {
    pub fn new(
        user: &User,
        show_access_levels: AccessLevelFormat,
        show_bot_icon: bool,
        registry: &dyn metadata::Registry,
        enabled: &[Metadata],
        truncate: Option<u16>,
        truncation_character: char,
        brackets: Option<&Brackets>,
        with_tooltip: bool,
        previews: Option<&'av preview::Collection>,
    ) -> Self {
        let query = Query::from(user);

        let color = if enabled.contains(&Metadata::Color) {
            registry.color(&query)
        } else {
            None
        };

        let full = UserDisplayData::new(
            user,
            query,
            show_access_levels,
            show_bot_icon,
            registry,
            enabled,
            previews,
        );

        if let Some(truncated) = truncate.and_then(|truncation_length| {
            full.truncate(truncation_length as usize, truncation_character)
        }) {
            Self {
                base: truncated.bracket(brackets),
                tooltip: with_tooltip.then_some(full),
                color,
            }
        } else if full.bot_icon && brackets.is_some() {
            Self {
                base: full.clone().bracket(brackets),
                tooltip: with_tooltip.then_some(full),
                color,
            }
        } else {
            let tooltip =
                (with_tooltip && full.bot_icon).then_some(full.clone());

            Self {
                base: full.bracket(brackets),
                tooltip,
                color,
            }
        }
    }

    pub fn into_element<'a, M: 'a>(
        self,
        user: &User,
        is_away: bool,
        is_offline: bool,
        dimmed: Option<(Dimmed, Color)>,
        size: Option<f32>,
        highlight: bool,
        selectable: bool,
        theme: &'a Theme,
        config: &'a Config,
    ) -> Element<'a, M> {
        let color = self.color.map(|color| {
            config.display.adapt_metadata_colors.adapt(
                color,
                theme.styles().buffer.nickname.color,
                theme.styles().buffer.background,
            )
        });

        let base = self.base.into_element(
            user,
            color,
            is_away,
            is_offline,
            dimmed,
            size,
            selectable,
            theme,
            config,
            font::line_height(),
        );

        let base = if highlight {
            let highlight_color = theme.styles().buffer.highlight;
            container(base)
                .style(move |_| iced::widget::container::Style {
                    background: Some(iced::Background::Color(highlight_color)),
                    ..Default::default()
                })
                .into()
        } else {
            base
        };

        if let Some(tooltip) = self.tooltip {
            iced::widget::tooltip(
                base,
                container(container(if tooltip.bot_icon {
                    row![
                        tooltip.into_element(
                            user,
                            color,
                            false,
                            false,
                            None,
                            None,
                            true,
                            theme,
                            config,
                            iced::widget::text::LineHeight::Relative(1.0),
                        ),
                        text(String::from(" is marked as a bot"))
                            .style(theme::text::secondary)
                            .line_height(
                                iced::widget::text::LineHeight::Relative(1.0),
                            )
                    ]
                    .spacing(theme::ICON_SPACE)
                    .into()
                } else {
                    tooltip.into_element(
                        user,
                        color,
                        false,
                        false,
                        None,
                        None,
                        true,
                        theme,
                        config,
                        iced::widget::text::LineHeight::Relative(1.0),
                    )
                }))
                .style(theme::container::tooltip)
                .padding(8),
                iced::widget::tooltip::Position::Top,
            )
            .delay(iced::time::Duration::ZERO)
            .into()
        } else {
            base
        }
    }

    pub fn width(&self, config: &Config) -> f32 {
        self.base.width(config)
    }
}

#[derive(Clone)]
pub struct UserDisplayData<'a> {
    left: String,
    bot_icon: bool,
    right: Option<String>,
    avatar: Option<buffer::context_menu::UserAvatar<'a>>,
}

impl <'av> UserDisplayData<'av> {
    pub fn new(
        user: &User,
        query: Query,
        show_access_levels: AccessLevelFormat,
        show_bot_icon: bool,
        registry: &dyn metadata::Registry,
        enabled: &[Metadata],
        previews: Option<&'av preview::Collection>,
    ) -> Self {
        let access_levels = match show_access_levels {
            AccessLevelFormat::All => {
                let access_levels = user
                    .access_levels()
                    .filter_map(AccessLevel::char)
                    .collect::<String>();

                if access_levels.is_empty() {
                    None
                } else {
                    Some(access_levels)
                }
            }
            AccessLevelFormat::Highest => {
                user.highest_access_level().char().map(String::from)
            }
            AccessLevelFormat::None => None,
        }
        .unwrap_or_default();

        let nickname = user.nickname();

        let bot_icon = user.is_bot() && show_bot_icon;

        let display_name = if enabled.contains(&Metadata::DisplayName)
            && let Some(display_name) =
                registry.display_name(TargetRef::Query(&query))
            && !display_name.is_empty()
        {
            Some(display_name)
        } else {
            None
        };

        let pronouns = if enabled.contains(&Metadata::Pronouns)
            && let Some(pronouns) = registry.pronouns(&query)
            && !pronouns.is_empty()
        {
            Some(pronouns)
        } else {
            None
        };

        let avatar: Option<buffer::context_menu::UserAvatar<'av>> = if let Some(previews) = previews {
            buffer::context_menu::user_avatar(user, registry, &previews)
        } else {
            None
        };

        if bot_icon {
            let (left, right) = if let Some(display_name) = display_name {
                (
                    format!("{display_name} ({access_levels}{nickname}"),
                    if let Some(pronouns) = pronouns {
                        Some(format!("{pronouns})"))
                    } else {
                        Some(String::from(")"))
                    },
                )
            } else {
                (
                    format!("{access_levels}{nickname}"),
                    pronouns.map(|pronouns| format!(" ({pronouns})")),
                )
            };

            Self {
                left,
                bot_icon,
                right,
                avatar,
            }
        } else {
            let left = match (display_name, pronouns) {
                (Some(display_name), Some(pronouns)) => {
                    format!(
                        "{display_name} ({access_levels}{nickname}, {pronouns})",
                    )
                }
                (Some(display_name), None) => {
                    format!("{display_name} ({access_levels}{nickname})",)
                }
                (None, Some(pronouns)) => {
                    format!("{access_levels}{nickname} ({pronouns})",)
                }
                (None, None) => format!("{access_levels}{nickname}",),
            };

            Self {
                left,
                bot_icon,
                right: None,
                avatar,
            }
        }
    }

    pub fn into_element<'a, M: 'a>(
        self,
        user: &User,
        color: Option<Color>,
        is_away: bool,
        is_offline: bool,
        dimmed: Option<(Dimmed, Color)>,
        size: Option<f32>,
        selectable: bool,
        theme: &'a Theme,
        config: &'a Config,
        line_height: iced::widget::text::LineHeight,
    ) -> Element<'a, M> {
        let style = theme::selectable_text::dimmed(
            theme::selectable_text::nickname(
                theme, config, user, color, is_away, is_offline,
            ),
            theme,
            dimmed,
        );

        self.render(style, is_offline, size, selectable, theme, line_height)
    }

    fn render<'a, M: 'a>(
        self,
        style: crate::widget::selectable_text::Style,
        is_offline: bool,
        size: Option<f32>,
        selectable: bool,
        theme: &'a Theme,
        line_height: iced::widget::text::LineHeight,
    ) -> Element<'a, M> {
        let font =
            theme::font_style::nickname(theme, is_offline).map(font::get);

        
        let avatar: Option<Element<'a, M>> = self.avatar.clone().map(|avatar| {
            let content: Element<'a, M> = match avatar {
                buffer::context_menu::UserAvatar::Pending => 
                    center(icon::people().size(16).style(theme::text::secondary))
                        .width(Length::Fixed(f32::from(AVATAR_SIZE)))
                        .height(Length::Fixed(f32::from(AVATAR_SIZE)))
                        .style(|theme| {
                            let general = theme.styles().general;
                            let text = theme.styles().text;

                            container::Style {
                                background: Some(Background::Color(general.background)),
                                border: Border {
                                    radius: 4.0.into(),
                                    width: 0.5,
                                    color: text.secondary.color,
                                },
                                ..Default::default()
                            }
                        })
                        .into(),
                buffer::context_menu::UserAvatar::Loaded(image_data) => {
                    container(widget::image::from_data(image_data, true, ContentFit::Cover))
                        .width(f32::from(AVATAR_SIZE))
                        .height(f32::from(AVATAR_SIZE))
                        .into()
                },
            };

            container(content)
                .width(Length::Fixed(f32::from(AVATAR_SIZE)))
                .height(Length::Fixed(f32::from(AVATAR_SIZE)))
                .into()
        });

        // selectable_text carries selection state and handles copy interactions;
        // plain text is used where selection would be undesirable (e.g. input bar)
        let text_piece =
            |content: String, f: Option<font::Font>| -> Element<'a, M> {
                if selectable {
                    selectable_text(content)
                        .style(move |_| style)
                        .font_maybe(f)
                        .size_maybe(size)
                        .line_height(line_height)
                        .wrapping(Wrapping::None)
                        .into()
                } else {
                    widget::text(content)
                        .color_maybe(style.color)
                        .font_maybe(f)
                        .size_maybe(size)
                        .line_height(line_height)
                        .wrapping(Wrapping::None)
                        .into()
                }
            };

        if self.bot_icon {
            let icon: Element<M> = if selectable {
                widget::bot_icon(move |_| style)
            } else {
                widget::text(String::from("\u{1F916}"))
                    .color_maybe(style.color)
                    .line_height(line_height)
                    .font(*font::ICON)
                    .size(theme::ICON_SIZE)
                    .into()
            };
            row![
                text_piece(self.left, font.clone()),
                icon,
                self.right.map(|right| text_piece(right, font)),
            ]
            .spacing(theme::ICON_SPACE)
            .align_y(Vertical::Center)
            .into()
        } else {
            if let Some(avatar) = avatar {
                row![
                    avatar,
                    text_piece(self.left, font),
                ].into()
            } else {
                text_piece(self.left, font)
            }
        }
    }

    pub fn width(&self, config: &Config) -> f32 {
        let mut width = font::width_from_str(self.left.as_str(), &config.font);

        if self.bot_icon {
            width += theme::ICON_SPACE + theme::ICON_SIZE;

            if let Some(right) = self.right.as_ref() {
                width += theme::ICON_SPACE
                    + font::width_from_str(right.as_str(), &config.font);
            }
        }

        width
    }

    pub fn truncate(
        &self,
        truncation_length: usize,
        truncation_character: char,
    ) -> Option<Self> {
        let left_length =
            UnicodeSegmentation::graphemes(self.left.as_str(), true).count();

        if truncation_length < left_length {
            return Some(Self {
                left: format!(
                    "{}{truncation_character}",
                    UnicodeSegmentation::graphemes(self.left.as_str(), true)
                        .take(truncation_length.saturating_sub(1))
                        .collect::<String>()
                ),
                bot_icon: false,
                right: None,
                avatar: self.avatar.clone(),
            });
        } else if self.bot_icon {
            if truncation_length < left_length.saturating_add(2) {
                return Some(Self {
                    left: format!(
                        "{}{truncation_character}",
                        self.left.as_str()
                    ),
                    bot_icon: false,
                    right: None,
                    avatar: self.avatar.clone(),
                });
            } else {
                let right_length = self
                    .right
                    .as_ref()
                    .map(|right| {
                        UnicodeSegmentation::graphemes(right.as_str(), true)
                            .count()
                    })
                    .unwrap_or_default();

                if truncation_length
                    < left_length.saturating_add(2).saturating_add(right_length)
                {
                    return Some(Self {
                        left: self.left.clone(),
                        bot_icon: true,
                        right: self.right.as_ref().map(|right| {
                            format!(
                                "{}{truncation_character}",
                                UnicodeSegmentation::graphemes(
                                    right.as_str(),
                                    true
                                )
                                .take(
                                    truncation_length
                                        .saturating_sub(left_length)
                                        .saturating_sub(2)
                                        .saturating_sub(1)
                                )
                                .collect::<String>()
                            )
                        }),
                        avatar: self.avatar.clone()
                    });
                }
            }
        }

        None
    }

    pub fn bracket(self, brackets: Option<&Brackets>) -> Self {
        if let Some(brackets) = brackets {
            if self.bot_icon {
                UserDisplayData {
                    left: format!("{}{}", brackets.left, self.left),
                    bot_icon: true,
                    right: Some(
                        self.right
                            .map(|right| format!("{right}{}", brackets.right))
                            .unwrap_or(brackets.right.clone()),
                    ),
                    avatar: self.avatar
                }
            } else {
                UserDisplayData {
                    left: format!(
                        "{}{}{}",
                        brackets.left, self.left, brackets.right
                    ),
                    bot_icon: false,
                    right: None,
                    avatar: self.avatar,
                }
            }
        } else {
            self
        }
    }
}
