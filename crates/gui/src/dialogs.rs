//! The dialogs, laid out like the web UI's: panel, 18 px title, small
//! uppercase section headings, grey hints, buttons in the footer.

use crate::i18n::{text, tr};
use crate::style::{check, input as text_input, key, look_text, pick as pick_list, scroll, Look};
use iced::alignment::Vertical;
use iced::widget::{button, column, container, mouse_area, opaque, row, slider, space};
use iced::{Background, Border, Color, Element, Length};

use crate::style::*;
use faderust_core::names::device_label;

use crate::{App, Dialog, Edit, Message, Target, HIDE_AFTER, RELEASE};

fn edit(e: Edit) -> Message {
    Message::Edit(e)
}

fn close() -> Message {
    Message::Open(None)
}

fn h3(label: impl Into<String>) -> Element<'static, Message> {
    column![
        space().height(8),
        text(tr(&label.into()).to_uppercase())
            .size(12)
            .font(BOLD)
            .color(muted_color())
    ]
    .into()
}

fn hint(label: impl Into<String>) -> Element<'static, Message> {
    text(label.into()).size(12).color(muted_color()).into()
}

fn btn(label: impl Into<String>, msg: Message, primary: bool) -> Element<'static, Message> {
    let look = if primary {
        Look::Primary
    } else {
        Look::Secondary
    };
    button(text(label.into()).size(13).color(look_text(look)))
        .padding([6, 14])
        .on_press(msg)
        .style(key(look))
        .into()
}

/// A row of buttons of which one is on, like the web UI's segmented control.
fn segmented(items: Vec<(String, bool, Message)>) -> Element<'static, Message> {
    row(items.into_iter().map(|(label, on, msg)| {
        let look = if on { Look::Primary } else { Look::Secondary };
        button(text(label).size(13).color(look_text(look)))
            .padding([6, 14])
            .on_press(msg)
            .style(key(look))
            .into()
    }))
    .spacing(1)
    .into()
}

fn frame<'a>(
    title: String,
    width: f32,
    tall: bool,
    body: Element<'a, Message>,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let card = container(
        column![
            row![
                text(title).size(18).color(text_color()),
                space().width(Length::Fill),
                button(text("✕").size(12).color(text_color()).center())
                    .width(28)
                    .height(28)
                    .padding(0)
                    .on_press(close())
                    .style(key(Look::Secondary)),
            ]
            .align_y(Vertical::Center),
            scroll(container(body).padding(iced::Padding::default().right(28))).height(if tall {
                Length::Fixed(680.0)
            } else {
                Length::Shrink
            }),
            container(footer)
                .width(Length::Fill)
                .align_x(iced::alignment::Horizontal::Right),
        ]
        .spacing(12),
    )
    .padding(20)
    .width(width)
    .max_height(if tall { 780.0 } else { 640.0 })
    .style(|_| container::Style {
        background: Some(Background::Color(panel())),
        border: Border {
            color: border(),
            width: 1.0,
            radius: 4.0.into(),
        },
        ..container::Style::default()
    });
    // The backdrop swallows clicks; a click on it closes the dialog.
    opaque(
        mouse_area(
            container(opaque(card))
                .center(Length::Fill)
                .padding(24)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color {
                        a: 0.5,
                        ..Color::BLACK
                    })),
                    ..container::Style::default()
                }),
        )
        .on_press(close()),
    )
}

fn no_footer() -> Element<'static, Message> {
    space().width(0).height(0).into()
}

pub fn view(app: &App, d: Dialog) -> Element<'_, Message> {
    match d {
        Dialog::Fx(i) => fx(app, i),
        Dialog::Apps => apps(app),
        Dialog::Presets => presets(app),
        Dialog::Windows => windows(app),
        Dialog::Design => design(app),
        Dialog::Settings => settings(app),
        Dialog::Log => log(app),
        Dialog::Selftest => selftest(app),
    }
}

fn fx(app: &App, i: usize) -> Element<'_, Message> {
    let f = &app.fx[i];
    let lit = (app.strips[i].shown + 90.0).clamp(1.0, 89.0) as u16;
    let band = |b: usize, name: &'static str, hz: &'static str| {
        row![
            container(text(format!("{name} {hz}")).size(13).color(text_color())).width(150),
            slider(-12.0..=12.0, f.eq[b], move |v| edit(Edit::Eq(i, b, v))).step(0.5_f32),
            container(
                text(format!("{:+.1} dB", f.eq[b]))
                    .font(MONO)
                    .size(13)
                    .color(text_color())
            )
            .width(70)
            .align_right(Length::Fill),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
    };
    let body = column![
        h3("Rauschfilter"),
        check(f.denoise)
            .label("Hintergrundgeräusche entfernen")
            .on_toggle(move |v| edit(Edit::Denoise(i, v))),
        hint("Filtert Tastatur, Lüfter und Rauschen aus der Stimme."),
        h3("Low-Cut"),
        segmented(
            [0u16, 80, 120, 160]
                .into_iter()
                .map(|hz| (
                    if hz == 0 {
                        "Aus".into()
                    } else {
                        format!("{hz} Hz")
                    },
                    f.low_cut == hz,
                    edit(Edit::LowCut(i, hz))
                ))
                .collect()
        ),
        hint("Schneidet Brummen und Trittschall unten ab."),
        h3("EQ"),
        band(0, "Bass", "120 Hz"),
        band(1, "Präsenz", "3 kHz"),
        band(2, "Höhen", "8 kHz"),
        h3("Gate"),
        check(f.gate)
            .label("Stumm unter der Schwelle")
            .on_toggle(move |v| edit(Edit::Gate(i, v))),
        row![
            container(text("Schwelle").size(13).color(text_color())).width(150),
            slider(-90.0..=-10.0, f.gate_db, move |v| edit(Edit::GateDb(
                i,
                v.round()
            ))),
            container(
                text(format!("{} dB", f.gate_db))
                    .font(MONO)
                    .size(13)
                    .color(text_color())
            )
            .width(70)
            .align_right(Length::Fill),
        ]
        .spacing(10)
        .align_y(Vertical::Center),
        row![
            space().width(160),
            container(row![
                container(space())
                    .width(Length::FillPortion(lit))
                    .height(8)
                    .style(|_| container::Style {
                        background: Some(Background::Color(signal())),
                        ..container::Style::default()
                    }),
                space().width(Length::FillPortion(90 - lit)),
            ])
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(panel_2())),
                border: Border {
                    radius: 4.0.into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }),
        ],
        hint("Der Balken zeigt den Pegel, der Strich die Schwelle."),
    ]
    .spacing(8);
    frame(
        format!("{}: {}", tr("Effekte"), app.strips[i].label),
        520.0,
        false,
        body.into(),
        row![btn("Zurücksetzen", edit(Edit::ResetFx(i)), false),]
            .spacing(8)
            .into(),
    )
}

/// A device for a pick list: shown by its label, identified by `id`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Choice {
    pub id: String,
    pub label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

fn error_line(app: &App) -> Element<'static, Message> {
    if app.dialog_error.is_empty() {
        space().height(0).into()
    } else {
        text(app.dialog_error.clone())
            .size(13)
            .color(danger())
            .into()
    }
}

fn apps(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    match &app.apps_data {
        None => list = list.push(hint("Lade …")),
        Some(data) if !data.supported => {
            list = list.push(hint(
                "Ausgabegeräte pro Programm gibt es nur unter Windows.",
            ))
        }
        Some(data) => {
            if !data.devices.iter().any(|d| d.cable) {
                list = list.push(
                    container(
                        text("Noch kein Faderust Kabel. Mit „+ Eingang“ nimmt Faderust ein Programm direkt auf. Damit es dann nicht zusätzlich über die Boxen läuft, braucht es ein Gerät, das du nicht hörst: das Faderust Kabel (Einrichtung unter Geräte).")
                            .size(13)
                            .color(text_color()),
                    )
                    .padding([10, 12])
                    .style(|_| container::Style {
                        border: Border {
                            color: accent(),
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        ..container::Style::default()
                    }),
                );
            }
            for cable in app.unused_cables() {
                list = list.push(
                    row![
                        text(format!(
                            "{} {}",
                            device_label(&cable),
                            tr("wird von keinem Eingang genutzt.")
                        ))
                        .size(13)
                        .color(text_color()),
                        space().width(Length::Fill),
                        btn(
                            "Als Eingang anlegen",
                            edit(Edit::AppInput(cable.clone(), device_label(&cable))),
                            true
                        ),
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
            }
            let quiet = app.quiet_cable();
            if data.apps.is_empty() {
                list = list.push(hint(
                    "Gerade spielt kein Programm Ton ab. Starte die Wiedergabe und tippe auf „Aktualisieren“.",
                ));
            }
            let default_choice = Choice {
                id: String::new(),
                label: "Windows-Standard".into(),
            };
            for a in &data.apps {
                let mut choices = vec![default_choice.clone()];
                choices.extend(data.devices.iter().map(|d| Choice {
                    id: d.id.clone(),
                    label: if d.cable {
                        device_label(&d.name)
                    } else {
                        d.name.clone()
                    },
                }));
                let current = match &a.device {
                    None => default_choice.clone(),
                    Some(id) => choices
                        .iter()
                        .find(|c| &c.id == id)
                        .cloned()
                        .unwrap_or(Choice {
                            id: id.clone(),
                            label: "Gerät nicht verbunden".into(),
                        }),
                };
                if !choices.contains(&current) {
                    choices.push(current.clone());
                }
                let pids = a.pids.clone();
                let device = pick_list(choices, Some(current), move |c| {
                    edit(Edit::AppDevice(
                        pids.clone(),
                        (!c.id.is_empty()).then_some(c.id),
                    ))
                })
                .width(Length::Fill);
                let program = format!("app:{}", a.exe.to_lowercase());
                let action: Element<'_, Message> = if app.input_devices.contains(&program) {
                    container(text("Eingang").size(12).font(BOLD).color(signal()))
                        .padding([5, 10])
                        .style(|_| container::Style {
                            border: Border {
                                color: signal(),
                                width: 1.0,
                                radius: 3.0.into(),
                            },
                            ..container::Style::default()
                        })
                        .into()
                } else if let Some(q) = &quiet {
                    // Park it on the quiet cable first, so it is not heard twice.
                    btn(
                        "+ Eingang",
                        edit(Edit::AppInputParked(
                            program,
                            a.name.clone(),
                            a.pids.clone(),
                            q.id.clone(),
                        )),
                        false,
                    )
                } else {
                    btn(
                        "+ Eingang",
                        edit(Edit::AppInput(program, a.name.clone())),
                        false,
                    )
                };
                list = list.push(
                    row![
                        container(text(a.name.clone()).size(14).color(text_color())).width(170),
                        device,
                        action
                    ]
                    .spacing(8)
                    .align_y(Vertical::Center),
                );
                if app.doubled(a) {
                    let fix: Element<'_, Message> = match &quiet {
                        Some(q) => btn(
                            format!("{} {}", tr("Auf"), device_label(&q.name)) + &tr(" stellen"),
                            edit(Edit::AppDevice(a.pids.clone(), Some(q.id.clone()))),
                            true,
                        ),
                        None => btn("Faderust Kabel einrichten", edit(Edit::UsbSetup), true),
                    };
                    list = list.push(
                        container(
                            row![
                                text(format!(
                                    "{} {}",
                                    tr("Du hörst"),
                                    a.name
                                ) + &tr(" doppelt: einmal über den Fader, einmal über sein eigenes Gerät."))
                                .size(12)
                                .color(amber()),
                                space().width(Length::Fill),
                                fix,
                            ]
                            .spacing(8)
                            .align_y(Vertical::Center),
                        )
                        .padding([6, 10])
                        .style(|_| container::Style {
                            border: Border {
                                color: amber(),
                                width: 1.0,
                                radius: 3.0.into(),
                            },
                            ..container::Style::default()
                        }),
                    );
                }
            }
        }
    }
    let parked_hint: Element<'_, Message> = match app.quiet_cable() {
        Some(q) => hint(format!(
            "{} {} {}",
            tr("„+ Eingang“ nimmt das Programm auf und stellt es vorher auf"),
            device_label(&q.name),
            tr(", damit du es nicht doppelt hörst.")
        )),
        None => space().height(0).into(),
    };
    let body = column![
        error_line(app),
        list,
        parked_hint,
        text("Windows merkt sich die Zuordnung pro Programm. Manche Programme übernehmen sie erst nach einem Neustart.")
            .size(12)
            .color(muted_color()),
    ]
    .spacing(16);
    frame(
        "Programme".into(),
        640.0,
        false,
        body.into(),
        row![btn("Aktualisieren", edit(Edit::Refresh), false),]
            .spacing(8)
            .into(),
    )
}

fn presets(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    for p in &app.presets {
        list = list.push(
            row![
                container(text(p.clone()).size(14).color(text_color())).width(Length::Fill),
                btn("Laden", edit(Edit::LoadPreset(p.clone())), true),
                btn("Löschen", edit(Edit::DeletePreset(p.clone())), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let body = column![
        h3("Gespeicherte Presets"),
        list,
        h3("Aktuelle Einstellung speichern"),
        row![
            text_input("Name", &app.new_preset)
                .on_input(|t| edit(Edit::NewPreset(t)))
                .on_submit(edit(Edit::SavePreset)),
            btn("Speichern", edit(Edit::SavePreset), true),
        ]
        .spacing(8),
        hint("Ein Preset merkt sich Fader, Stumm und Effekte aller Eingänge."),
    ]
    .spacing(10);
    frame("Presets".into(), 460.0, false, body.into(), no_footer())
}

fn windows(app: &App) -> Element<'_, Message> {
    let mut body = column![error_line(app)].spacing(10);
    match &app.win {
        None => body = body.push(hint("Lade …")),
        Some(w) if !w.supported => {
            body = body.push(hint(
                "Die Windows-Soundeinstellungen gibt es nur unter Windows.",
            ))
        }
        Some(w) => {
            for (title, list) in [("Wiedergabe", &w.playback), ("Aufnahme", &w.recording)] {
                body = body.push(h3(title));
                let choices: Vec<Choice> = list
                    .iter()
                    .map(|e| Choice {
                        id: e.id.clone(),
                        label: device_label(&e.name),
                    })
                    .collect();
                let pick = |label: &'static str, comm: bool| {
                    let current = list
                        .iter()
                        .find(|e| {
                            if comm {
                                e.is_default_communications
                            } else {
                                e.is_default
                            }
                        })
                        .and_then(|e| choices.iter().find(|c| c.id == e.id).cloned());
                    row![
                        container(text(label).size(13).color(text_color())).width(130),
                        pick_list(choices.clone(), current, move |c| edit(Edit::WinDefault(
                            c.id, comm
                        )))
                        .width(Length::Fill),
                    ]
                    .spacing(10)
                    .align_y(Vertical::Center)
                };
                body = body
                    .push(pick("Standardgerät", false))
                    .push(pick("Für Anrufe", true));
                for e in list {
                    let id = e.id.clone();
                    let badge = if e.is_default {
                        "  Standard"
                    } else if e.is_default_communications {
                        "  Anrufe"
                    } else {
                        ""
                    };
                    let mute_id = id.clone();
                    let muted = e.muted;
                    body = body.push(
                        row![
                            container(
                                text(format!("{}{badge}", device_label(&e.name)))
                                    .size(13)
                                    .color(text_color())
                            )
                            .width(230),
                            slider(0.0..=100.0, e.volume * 100.0, move |v| edit(
                                Edit::WinVolume(id.clone(), v)
                            )),
                            container(
                                text(format!("{} %", (e.volume * 100.0).round()))
                                    .font(MONO)
                                    .size(13)
                                    .color(text_color())
                            )
                            .width(54)
                            .align_right(Length::Fill),
                            button(text(if muted { "Stumm" } else { "Laut" }).size(12).color(
                                look_text(if muted { Look::Danger } else { Look::Secondary })
                            ))
                            .padding([4, 8])
                            .width(64)
                            .on_press(edit(Edit::WinMute(mute_id, !muted)))
                            .style(key(if muted {
                                Look::Danger
                            } else {
                                Look::Secondary
                            })),
                        ]
                        .spacing(10)
                        .align_y(Vertical::Center),
                    );
                }
            }
            body = body.push(hint(
                "Faderust nimmt Geräte direkt ab. Der Windows-Standard bestimmt nur, wohin Programme ohne eigene Zuordnung spielen und welches Mikrofon sie nutzen.",
            ));
        }
    }
    frame(
        "Windows-Sound".into(),
        700.0,
        false,
        body.into(),
        row![btn("Aktualisieren", edit(Edit::Refresh), false),]
            .spacing(8)
            .into(),
    )
}

fn design(app: &App) -> Element<'_, Message> {
    let swatch = |hex: &'static str, on: bool, msg: Message| {
        let c = hex_color(hex);
        button(space())
            .width(28)
            .height(28)
            .style(move |_, _| button::Style {
                background: Some(Background::Color(c)),
                border: Border {
                    color: if on { text_color() } else { border() },
                    width: if on { 3.0 } else { 1.0 },
                    radius: 14.0.into(),
                },
                ..button::Style::default()
            })
            .on_press(msg)
    };
    let hide = HIDE_AFTER
        .iter()
        .map(|h| {
            (
                h.to_string(),
                app.hide_after == *h,
                edit(Edit::HideAfter(h)),
            )
        })
        .collect();
    let accent_now = app.accent();
    let signal_now = app.signal();
    let mode_now = app.mode();
    let accents = ACCENTS.iter().fold(row![].spacing(8), |r, (_, _, hex)| {
        r.push(swatch(
            hex,
            accent_now.eq_ignore_ascii_case(hex),
            edit(Edit::Accent((*hex).into())),
        ))
    });
    let signals = SIGNALS.iter().fold(row![].spacing(8), |r, (_, _, hex)| {
        r.push(swatch(
            hex,
            signal_now.eq_ignore_ascii_case(hex),
            edit(Edit::Signal((*hex).into())),
        ))
    });
    let lang_now = app.lang();
    let body = column![
        h3("Ansicht"),
        segmented(vec![
            (
                "Pult".into(),
                matches!(app.view, crate::View::Desk),
                Message::SetView(crate::View::Desk)
            ),
            (
                "Matrix".into(),
                matches!(app.view, crate::View::Matrix),
                Message::SetView(crate::View::Matrix)
            ),
        ]),
        h3("Ausgänge"),
        segmented(vec![
            (
                "Links".into(),
                app.masters_left,
                edit(Edit::MastersLeft(true))
            ),
            (
                "Rechts".into(),
                !app.masters_left,
                edit(Edit::MastersLeft(false))
            ),
        ]),
        h3("Stille Programme ausblenden"),
        segmented(hide),
        h3("Akzentfarbe"),
        accents,
        custom_color(
            app.accent_input
                .clone()
                .unwrap_or_else(|| accent_now.clone()),
            Target::Accent,
            app.picker,
            |t| edit(Edit::AccentHex(t)),
        ),
        h3("Signalfarbe"),
        signals,
        custom_color(
            app.signal_input
                .clone()
                .unwrap_or_else(|| signal_now.clone()),
            Target::Signal,
            app.picker,
            |t| edit(Edit::SignalHex(t)),
        ),
        h3("Helligkeit"),
        segmented(vec![
            (
                "Dunkel".into(),
                mode_now == "dark",
                edit(Edit::Mode("dark"))
            ),
            (
                "Hell".into(),
                mode_now == "light",
                edit(Edit::Mode("light"))
            ),
            (
                "Wie Windows".into(),
                mode_now == "system",
                edit(Edit::Mode("system"))
            ),
        ]),
        h3("Sprache"),
        segmented(vec![
            ("Deutsch".into(), lang_now == "de", edit(Edit::Lang("de"))),
            ("English".into(), lang_now == "en", edit(Edit::Lang("en"))),
        ]),
        h3("Darstellung"),
        segmented(vec![
            (
                "Grafikkarte".into(),
                !app.prefs.software,
                edit(Edit::Software(false))
            ),
            (
                "Software".into(),
                app.prefs.software,
                edit(Edit::Software(true))
            ),
        ]),
        hint("Grafikkarte braucht wenig Prozessor, Software wenig Arbeitsspeicher. Gilt nach einem Neustart von Faderust."),
    ]
    .spacing(10);
    frame("Design".into(), 480.0, true, body.into(), no_footer())
}

pub fn tip<'a>(
    content: impl Into<Element<'a, Message>>,
    tip: impl Into<String>,
) -> Element<'a, Message> {
    iced::widget::tooltip(
        content,
        container(text(tip.into()).size(12).color(text_color()))
            .padding(6)
            .style(|_| container::Style {
                background: Some(Background::Color(panel_2())),
                border: Border {
                    color: border(),
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..container::Style::default()
            }),
        iced::widget::tooltip::Position::Bottom,
    )
    .into()
}

/// Any colour: a picker (opened by the swatch) and a field for #rrggbb.
fn custom_color(
    value: String,
    target: Target,
    picker: Option<(Target, (f32, f32, f32))>,
    on_input: impl Fn(String) -> Message + 'static,
) -> Element<'static, Message> {
    let valid = crate::style::parse_hex_input(&value);
    let preview = valid.as_deref().map(crate::style::hex);
    let open = picker.filter(|(t, _)| *t == target);
    let swatch = button(space())
        .width(34)
        .height(28)
        .padding(0)
        .on_press(edit(Edit::Picker(target)))
        .style(move |_, status| button::Style {
            background: Some(Background::Color(preview.unwrap_or(Color::TRANSPARENT))),
            border: Border {
                color: if open.is_some() || matches!(status, button::Status::Hovered) {
                    accent_light()
                } else if preview.is_some() {
                    border()
                } else {
                    danger()
                },
                width: 2.0,
                radius: 6.0.into(),
            },
            ..button::Style::default()
        });
    let head = row![
        text("Eigene Farbe").size(13).color(muted_color()),
        swatch,
        text_input("#rrggbb", &value).on_input(on_input).width(100),
        btn(
            if open.is_some() {
                "Farbwähler schließen"
            } else {
                "Farbwähler"
            },
            edit(Edit::Picker(target)),
            false
        ),
    ]
    .spacing(10)
    .align_y(Vertical::Center);
    match open {
        Some((_, hsv)) => column![
            head,
            iced::widget::canvas(crate::picker::Picker { hsv, target })
                .width(crate::picker::W)
                .height(crate::picker::H),
        ]
        .spacing(10)
        .into(),
        None => head.into(),
    }
}

fn hex_color(h: &str) -> Color {
    crate::style::hex(h)
}

fn deck_section(app: &App) -> Element<'_, Message> {
    let Some(deck) = &app.deck else {
        return hint("Lade …");
    };
    let Some(bundled) = &deck.bundled else {
        return hint("Dieses Faderust enthält kein Stream-Deck-Plugin.");
    };
    let label = if deck.refused {
        "Das Plugin ist zu alt und darf sich nicht mehr verbinden. Bitte aktualisieren:".to_string()
    } else if !deck.connected {
        "Stream Deck nicht verbunden. Plugin einrichten oder neu installieren:".to_string()
    } else if deck.update {
        format!(
            "Plugin {} → {bundled} {}",
            deck.installed.as_deref().unwrap_or("(alt)"),
            tr("verfügbar")
        )
    } else {
        format!(
            "Plugin {} {}",
            deck.installed.as_deref().unwrap_or(""),
            tr("ist aktuell")
        )
    };
    let action = if deck.connected && !deck.update {
        None
    } else if deck.connected || deck.refused {
        Some("Plugin aktualisieren")
    } else {
        Some("Plugin installieren")
    };
    let mut r = row![hint(label), space().width(Length::Fill)]
        .spacing(8)
        .align_y(Vertical::Center);
    if let Some(a) = action {
        r = r.push(btn(a, edit(Edit::InstallDeck), false));
    }
    column![r, error_text(&app.deck_error)].spacing(6).into()
}

fn error_text(e: &str) -> Element<'static, Message> {
    if e.is_empty() {
        space().height(0).into()
    } else {
        text(e.to_string()).size(13).color(danger()).into()
    }
}

fn log(app: &App) -> Element<'_, Message> {
    let text_ = match &app.log_text {
        None => "Lade …".to_string(),
        Some(t) if t.trim().is_empty() => "Noch nichts protokolliert.".to_string(),
        Some(t) => t.clone(),
    };
    let body = column![
        hint("Was Faderust zuletzt gemeldet hat. Bei einem Fehler bitte den Text kopieren und in ein Issue einfügen."),
        container(text(text_).font(MONO).size(12).color(text_color()))
            .padding(10)
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(bg())),
                border: Border {
                    color: border(),
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..container::Style::default()
            }),
    ]
    .spacing(10);
    frame(
        "Protokoll".into(),
        900.0,
        true,
        body.into(),
        row![
            btn("Aktualisieren", edit(Edit::Refresh), false),
            btn("Kopieren", edit(Edit::CopyLog), false),
            btn("Ordner öffnen", edit(Edit::OpenLogFolder), false),
        ]
        .spacing(8)
        .into(),
    )
}

fn selftest(app: &App) -> Element<'_, Message> {
    use faderust_core::selftest::Status;
    let mut body = column![
        error_line(app),
        hint("Prüft Geräte, Puffer, Fernsteuerung und Plugin. Der Bericht lässt sich in ein Issue kopieren."),
    ]
    .spacing(10);
    match &app.checks {
        None => body = body.push(hint("Prüfe …")),
        Some(checks) => {
            let problems = checks
                .iter()
                .filter(|c| matches!(c.status, Status::Warn | Status::Fail))
                .count();
            body = body.push(
                text(if problems == 0 {
                    "Alles in Ordnung".to_string()
                } else {
                    format!("{problems} {}", tr("Punkte brauchen einen Blick"))
                })
                .size(15)
                .color(if problems == 0 { signal() } else { amber() }),
            );
            for c in checks {
                let (mark, color) = match c.status {
                    Status::Ok => ("✓", signal()),
                    Status::Warn => ("!", amber()),
                    Status::Fail => ("✕", danger()),
                    Status::Info => ("i", muted_color()),
                };
                body = body.push(
                    row![
                        container(text(mark).size(14).font(BOLD).color(color)).width(20),
                        column![
                            text(c.title.clone())
                                .size(13)
                                .font(BOLD)
                                .color(text_color()),
                            text(c.detail.clone()).size(12).color(muted_color()),
                        ]
                        .spacing(2),
                    ]
                    .spacing(8),
                );
            }
        }
    }
    frame(
        "Selbsttest".into(),
        700.0,
        false,
        body.into(),
        row![
            btn("Nochmal prüfen", edit(Edit::Refresh), false),
            btn("Bericht kopieren", edit(Edit::CopyChecks), false),
        ]
        .spacing(8)
        .into(),
    )
}

fn settings(app: &App) -> Element<'_, Message> {
    let Some(draft) = &app.draft else {
        return frame("Geräte".into(), 720.0, true, hint("Lade …"), no_footer());
    };
    let device_choices = |list: &[faderust_core::Device], current: &str| -> (Vec<Choice>, Choice) {
        let mut choices: Vec<Choice> = list
            .iter()
            .map(|d| Choice {
                id: d.name.clone(),
                label: device_label(&d.name),
            })
            .collect();
        let lower = current.to_lowercase();
        let selected = list
            .iter()
            .find(|d| d.id == current)
            .or_else(|| list.iter().find(|d| d.name.to_lowercase().contains(&lower)))
            .map(|d| Choice {
                id: d.name.clone(),
                label: device_label(&d.name),
            })
            .unwrap_or(Choice {
                id: current.to_string(),
                label: device_label(current),
            });
        // A configured device that is unplugged right now stays selectable.
        if !choices.contains(&selected) {
            choices.insert(0, selected.clone());
        }
        (choices, selected)
    };
    let arrow = |label: &'static str, msg: Option<Message>| {
        let b = button(text(label).size(13).color(text_color()))
            .padding([4, 8])
            .style(key(Look::Secondary));
        match msg {
            Some(m) => b.on_press(m),
            None => b,
        }
    };
    let program = |d: &str| d.starts_with("app:");
    let last_dev = draft.inputs.iter().rposition(|i| !program(&i.device));
    let first_dev = draft.inputs.iter().position(|i| !program(&i.device));
    let mut inputs = column![].spacing(8);
    for (i, input) in draft.inputs.iter().enumerate() {
        if program(&input.device) {
            continue;
        }
        let (choices, current) = device_choices(&app.devs.inputs, &input.device);
        let prev = draft.inputs[..i].iter().rposition(|x| !program(&x.device));
        let next = draft.inputs[i + 1..]
            .iter()
            .position(|x| !program(&x.device))
            .map(|p| p + i + 1);
        inputs = inputs.push(
            row![
                text_input("Name", &input.label)
                    .on_input(move |t| edit(Edit::InLabel(i, t)))
                    .width(140),
                pick_list(choices, Some(current), move |c| edit(Edit::InDevice(
                    i, c.id
                )))
                .width(Length::Fill),
                arrow(
                    "↑",
                    (first_dev != Some(i)).then(|| edit(Edit::InMove(
                        i,
                        prev.map_or(0, |p| p as isize - i as isize)
                    )))
                ),
                arrow(
                    "↓",
                    (last_dev != Some(i)).then(|| edit(Edit::InMove(
                        i,
                        next.map_or(0, |n| n as isize - i as isize)
                    )))
                ),
                btn("✕", edit(Edit::InRemove(i)), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let mut outputs = column![].spacing(8);
    let n_out = draft.outputs.len();
    for (o, output) in draft.outputs.iter().enumerate() {
        let (choices, current) = device_choices(&app.devs.outputs, &output.device);
        let release = RELEASE
            .iter()
            .find(|(s, _)| *s == output.release_secs)
            .copied()
            .unwrap_or((output.release_secs, "Andere"));
        outputs = outputs.push(
            row![
                text_input("Name", &output.label)
                    .on_input(move |t| edit(Edit::OutLabel(o, t)))
                    .width(120),
                pick_list(choices, Some(current), move |c| edit(Edit::OutDevice(
                    o, c.id
                )))
                .width(Length::Fill),
                text_input("ms", &output.buffer_ms.to_string())
                    .on_input(move |t| edit(Edit::OutBuffer(o, t)))
                    .width(54),
                pick_list(
                    RELEASE.iter().map(|(_, n)| *n).collect::<Vec<_>>(),
                    Some(release.1),
                    move |n| edit(Edit::OutRelease(
                        o,
                        RELEASE
                            .iter()
                            .find(|(_, x)| *x == n)
                            .map_or(30, |(s, _)| *s)
                    ))
                ),
                arrow("←", (o > 0).then(|| edit(Edit::OutMove(o, -1)))),
                arrow("→", (o + 1 < n_out).then(|| edit(Edit::OutMove(o, 1)))),
                btn("✕", edit(Edit::OutRemove(o)), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let cable_section: Element<'_, Message> = match &app.usb {
        None => hint("Lade …"),
        Some(u) if !u.supported => hint("Das Faderust Kabel gibt es nur unter Windows."),
        Some(u) => {
            let mut col = column![hint(
                "Virtuelle Kabel für Programm-Mikro und Programme, die du nicht über die Boxen hören willst. Faderust stellt sie selbst als USB-Soundgerät bereit, ohne zusätzliche Treiber-Apps. Einmalige Einrichtung, dafür sind Administratorrechte nötig.",
            )]
            .spacing(10);
            if !u.installed {
                col = col.push(btn(
                    if app.usb_busy {
                        "Richte ein …"
                    } else {
                        "Faderust Kabel einrichten"
                    },
                    edit(Edit::UsbSetup),
                    true,
                ));
            } else {
                let count = u.count.clamp(1, 4) as u8;
                col = col
                    .push(
                        check(u.running)
                            .label("Faderust Kabel verwenden")
                            .on_toggle(|v| edit(Edit::UsbEnable(v))),
                    )
                    .push(
                        row![
                            text("Anzahl Kabel").size(13).color(text_color()),
                            pick_list([1u8, 2, 3, 4], Some(count), |n| edit(Edit::UsbCount(n))),
                            space().width(Length::Fill),
                            btn("Entfernen", edit(Edit::UsbRemove), false),
                        ]
                        .spacing(12)
                        .align_y(Vertical::Center),
                    );
                if u.running {
                    col = col.push(
                        text(
                            u.attached
                                .iter()
                                .enumerate()
                                .map(|(i, on)| {
                                    format!(
                                        "Faderust Kabel{}: {}",
                                        if i == 0 {
                                            String::new()
                                        } else {
                                            format!(" {}", i + 1)
                                        },
                                        if *on {
                                            "verbunden"
                                        } else {
                                            "verbinde mit Windows …"
                                        }
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                        .size(12)
                        .color(muted_color()),
                    );
                }
            }
            col.into()
        }
    };
    let body = column![
        error_line(app),
        h3("Eingänge"),
        inputs,
        btn("+ Eingang", edit(Edit::InAdd), false),
        h3("Ausgänge"),
        outputs,
        btn("+ Ausgang", edit(Edit::OutAdd), false),
        mic_section(app),
        h3("Start"),
        check(app.autostart)
            .label("Mit Windows starten (im Tray, ohne Fenster)")
            .on_toggle(|v| edit(Edit::Autostart(v))),
        h3("Faderust Kabel"),
        cable_section,
        h3("Stream Deck"),
        deck_section(app),
        h3("Selbsttest"),
        row![
            hint("Prüft Geräte, Puffer, Fernsteuerung und Plugin."),
            space().width(Length::Fill),
            btn("Starten", Message::Open(Some(Dialog::Selftest)), false),
            btn("Protokoll", Message::Open(Some(Dialog::Log)), false),
        ]
        .spacing(8)
        .align_y(Vertical::Center),
    ]
    .spacing(10);
    frame(
        "Geräte".into(),
        760.0,
        true,
        body.into(),
        hint("Änderungen werden automatisch gespeichert."),
    )
}

/// "Programm-Mikro": an output on a virtual cable, so Discord, Teams & co.
/// get Faderust's microphone mix by recording the cable's other end.
fn mic_section(app: &App) -> Element<'_, Message> {
    if let Some(device) = &app.mic_device {
        return column![
            text(tr("Programm-Mikro angelegt."))
                .size(13)
                .font(BOLD)
                .color(signal()),
            hint(format!(
                "{} {}. {}",
                tr("Wähle in Discord, Teams usw. als Mikrofon"),
                device_label(device),
                tr("Die Spalte „Programm-Mikro“ bestimmt, was dort ankommt.")
            )),
            check(app.mic_default)
                .label(tr("Als Windows-Mikrofon für Anrufe setzen"))
                .on_toggle(|v| edit(Edit::MicDefault(v))),
        ]
        .spacing(6)
        .into();
    }
    if app.mic_cable().is_some() {
        return row![
            btn("+ Programm-Mikro", edit(Edit::MicAdd), false),
            hint("Ausgang auf einem virtuellen Kabel, den Programme als Mikrofon nutzen"),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
        .into();
    }
    hint(if app.has_cable() {
        "Für ein Programm-Mikro (Discord, Teams …) braucht es ein freies virtuelles Kabel. Alle Kabel sind schon belegt: erhöhe die Anzahl unter „Faderust Kabel“."
    } else {
        "Für ein Programm-Mikro (Discord, Teams …) braucht es ein virtuelles Kabel."
    })
}
