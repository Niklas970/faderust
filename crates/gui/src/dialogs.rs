//! The dialogs, laid out like the web UI's: panel, 18 px title, small
//! uppercase section headings, grey hints, buttons in the footer.

use iced::alignment::Vertical;
use iced::widget::{
    button, checkbox, column, container, mouse_area, opaque, pick_list, row, scrollable, slider,
    space, text, text_input,
};
use iced::{Background, Border, Color, Element, Length};

use crate::style::*;
use crate::{flat, App, Dialog, Edit, Message, DEV_IN, DEV_OUT, HIDE_AFTER, RELEASE};

fn edit(e: Edit) -> Message {
    Message::Edit(e)
}

fn close() -> Message {
    Message::Open(None)
}

fn h3(label: &'static str) -> Element<'static, Message> {
    column![
        space().height(8),
        text(label.to_uppercase()).size(12).font(BOLD).color(MUTED)
    ]
    .into()
}

fn hint(label: &'static str) -> Element<'static, Message> {
    text(label).size(12).color(MUTED).into()
}

fn btn(label: &'static str, msg: Message, primary: bool) -> Element<'static, Message> {
    button(text(label).size(13).color(Color::WHITE))
        .padding([6, 12])
        .on_press(msg)
        .style(move |_, _| flat(if primary { ACCENT } else { PANEL_2 }))
        .into()
}

/// A row of buttons of which one is on, like the web UI's segmented control.
fn segmented(items: Vec<(String, bool, Message)>) -> Element<'static, Message> {
    row(items.into_iter().map(|(label, on, msg)| {
        button(text(label).size(13).color(Color::WHITE))
            .padding([6, 14])
            .on_press(msg)
            .style(move |_, _| flat(if on { ACCENT } else { PANEL_2 }))
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
            text(title).size(18).color(TEXT),
            scrollable(container(body).padding([0, 12])).height(if tall {
                Length::Fill
            } else {
                Length::Shrink
            }),
            container(footer).align_right(Length::Fill),
        ]
        .spacing(12),
    )
    .padding(20)
    .width(width)
    .max_height(if tall { 780.0 } else { 640.0 })
    .height(if tall { Length::Fill } else { Length::Shrink })
    .style(|_| container::Style {
        background: Some(Background::Color(PANEL)),
        border: Border {
            color: BORDER,
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

fn done(label: &'static str) -> Element<'static, Message> {
    btn(label, close(), true)
}

pub fn view(app: &App, d: Dialog) -> Element<'_, Message> {
    match d {
        Dialog::Fx(i) => fx(app, i),
        Dialog::Apps => apps(app),
        Dialog::Presets => presets(app),
        Dialog::Windows => windows(app),
        Dialog::Design => design(app),
        Dialog::Settings => settings(app),
    }
}

fn fx(app: &App, i: usize) -> Element<'_, Message> {
    let f = &app.fx[i];
    let lit = (app.strips[i].shown + 90.0).clamp(1.0, 89.0) as u16;
    let band = |b: usize, name: &'static str, hz: &'static str| {
        row![
            container(text(format!("{name} {hz}")).size(13).color(TEXT)).width(150),
            slider(-12.0..=12.0, f.eq[b], move |v| edit(Edit::Eq(i, b, v))).step(0.5_f32),
            container(
                text(format!("{:+.1} dB", f.eq[b]))
                    .font(MONO)
                    .size(13)
                    .color(TEXT)
            )
            .width(70)
            .align_right(Length::Fill),
        ]
        .spacing(10)
        .align_y(Vertical::Center)
    };
    let body = column![
        h3("Rauschfilter"),
        checkbox(f.denoise)
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
        checkbox(f.gate)
            .label("Stumm unter der Schwelle")
            .on_toggle(move |v| edit(Edit::Gate(i, v))),
        row![
            container(text("Schwelle").size(13).color(TEXT)).width(150),
            slider(-90.0..=-10.0, f.gate_db, move |v| edit(Edit::GateDb(
                i,
                v.round()
            ))),
            container(
                text(format!("{} dB", f.gate_db))
                    .font(MONO)
                    .size(13)
                    .color(TEXT)
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
                        background: Some(Background::Color(SIGNAL)),
                        ..container::Style::default()
                    }),
                space().width(Length::FillPortion(90 - lit)),
            ])
            .width(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(PANEL_2)),
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
        format!("Effekte: {}", app.strips[i].label),
        520.0,
        false,
        body.into(),
        row![
            btn("Zurücksetzen", edit(Edit::ResetFx(i)), false),
            done("Fertig")
        ]
        .spacing(8)
        .into(),
    )
}

fn apps(app: &App) -> Element<'_, Message> {
    let mut list = column![container(
        row![
            text("CABLE Output").size(13).color(TEXT),
            text("wird noch von keinem Eingang genutzt.")
                .size(13)
                .color(TEXT),
            space().width(Length::Fill),
            btn("Als Eingang", close(), true),
        ]
        .spacing(6)
        .align_y(Vertical::Center)
    )
    .padding([10, 12])
    .style(|_| container::Style {
        border: Border {
            color: ACCENT,
            width: 1.0,
            radius: 3.0.into()
        },
        ..container::Style::default()
    }),]
    .spacing(8);
    for (i, a) in app.apps.iter().enumerate() {
        let device = pick_list(crate::APP_OUT, Some(a.device), move |d| {
            edit(Edit::AppDevice(i, d))
        })
        .width(Length::Fill);
        let action: Element<'_, Message> = if a.input {
            container(text("Eingang").size(12).font(BOLD).color(SIGNAL))
                .padding([5, 10])
                .style(|_| container::Style {
                    border: Border {
                        color: SIGNAL,
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..container::Style::default()
                })
                .into()
        } else {
            btn("+ Eingang", edit(Edit::AppInput(i)), false)
        };
        list = list.push(
            row![
                container(text(a.name).size(14).color(TEXT)).width(170),
                device,
                action
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let body = column![
        list,
        text("Ein Programm auf einem Kabel erreicht Faderust über dessen Aufnahmeseite. Programme, die gerade nichts spielen, erscheinen erst, wenn sie Ton ausgeben.")
            .size(12)
            .color(MUTED),
    ]
    .spacing(16);
    frame(
        "Programme".into(),
        640.0,
        false,
        body.into(),
        row![btn("Aktualisieren", close(), false), done("Schließen")]
            .spacing(8)
            .into(),
    )
}

fn presets(app: &App) -> Element<'_, Message> {
    let mut list = column![].spacing(8);
    for p in &app.presets {
        list = list.push(
            row![
                container(text(p.clone()).size(14).color(TEXT)).width(Length::Fill),
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
    frame(
        "Presets".into(),
        460.0,
        false,
        body.into(),
        done("Schließen"),
    )
}

fn windows(app: &App) -> Element<'_, Message> {
    let devices = [
        "Lautsprecher (Realtek Audio)",
        "Kopfhörer (WH-1000XM5)",
        "CABLE Input (VB-Audio)",
    ];
    let mut list = column![].spacing(10);
    for (i, d) in devices.iter().enumerate() {
        let on = app.win_default == i;
        list = list.push(
            row![
                button(
                    text(if on { "Standard" } else { "Als Standard" })
                        .size(12)
                        .color(Color::WHITE)
                )
                .width(100)
                .padding([5, 8])
                .on_press(edit(Edit::WinDefault(i)))
                .style(move |_, _| flat(if on { ACCENT } else { PANEL_2 })),
                container(text(*d).size(13).color(TEXT)).width(210),
                slider(0.0..=100.0, app.win_volume[i], move |v| edit(
                    Edit::WinVolume(i, v)
                )),
                container(
                    text(format!("{} %", app.win_volume[i]))
                        .font(MONO)
                        .size(13)
                        .color(TEXT)
                )
                .width(54)
                .align_right(Length::Fill),
            ]
            .spacing(10)
            .align_y(Vertical::Center),
        );
    }
    let body = column![
        h3("Wiedergabe"),
        list,
        hint("Das sind die Windows-Einstellungen, nicht die Fader von Faderust."),
    ]
    .spacing(10);
    frame(
        "Windows-Sound".into(),
        640.0,
        false,
        body.into(),
        done("Schließen"),
    )
}

fn design(app: &App) -> Element<'_, Message> {
    let swatch = |c: Color, on: bool| {
        button(space())
            .width(28)
            .height(28)
            .style(move |_, _| button::Style {
                background: Some(Background::Color(c)),
                border: Border {
                    color: if on { Color::WHITE } else { BORDER },
                    width: if on { 2.0 } else { 1.0 },
                    radius: 14.0.into(),
                },
                ..button::Style::default()
            })
            .on_press(close())
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
        row![
            swatch(ACCENT, true),
            swatch(rgb(94, 154, 235), false),
            swatch(rgb(176, 120, 230), false),
            swatch(rgb(230, 190, 70), false),
            swatch(rgb(220, 90, 130), false),
        ]
        .spacing(8),
        h3("Signalfarbe"),
        row![
            swatch(SIGNAL, true),
            swatch(rgb(64, 200, 210), false),
            swatch(rgb(150, 220, 80), false),
            swatch(rgb(235, 235, 235), false),
        ]
        .spacing(8),
        h3("Helligkeit"),
        segmented(vec![
            ("Dunkel".into(), true, close()),
            ("Hell".into(), false, close()),
            ("Wie Windows".into(), false, close()),
        ]),
    ]
    .spacing(10);
    frame("Design".into(), 480.0, false, body.into(), done("Fertig"))
}

fn settings(app: &App) -> Element<'_, Message> {
    let s = &app.settings;
    let mut inputs = column![].spacing(8);
    for (i, (label, device)) in s.inputs.iter().enumerate() {
        inputs = inputs.push(
            row![
                text_input("Name", label)
                    .on_input(move |t| edit(Edit::InLabel(i, t)))
                    .width(140),
                pick_list(DEV_IN, Some(*device), move |d| edit(Edit::InDevice(i, d)))
                    .width(Length::Fill),
                btn("✕", close(), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let mut outputs = column![].spacing(8);
    for (o, (label, device, buf, rel)) in s.outputs.iter().enumerate() {
        outputs = outputs.push(
            row![
                text_input("Name", label)
                    .on_input(move |t| edit(Edit::OutLabel(o, t)))
                    .width(120),
                pick_list(DEV_OUT, Some(*device), move |d| edit(Edit::OutDevice(o, d)))
                    .width(Length::Fill),
                text_input("ms", buf)
                    .on_input(move |t| edit(Edit::OutBuffer(o, t)))
                    .width(54),
                pick_list(RELEASE, Some(*rel), move |r| edit(Edit::OutRelease(o, r))),
                btn("✕", close(), false),
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        );
    }
    let body = column![
        h3("Eingänge"),
        inputs,
        btn("+ Eingang", close(), false),
        h3("Ausgänge"),
        outputs,
        btn("+ Ausgang", close(), false),
        h3("Start"),
        checkbox(s.autostart)
            .label("Mit Windows starten")
            .on_toggle(|v| edit(Edit::Autostart(v))),
        h3("Faderust Kabel (Test)"),
        checkbox(app.usb_running)
            .label("Faderust Kabel verwenden")
            .on_toggle(|v| edit(Edit::UsbRunning(v))),
        row![
            text("Anzahl Kabel").size(13).color(TEXT),
            pick_list([1u8, 2, 3, 4], Some(app.usb_count), |n| edit(
                Edit::UsbCount(n)
            )),
            space().width(Length::Fill),
            btn("Entfernen", close(), false),
        ]
        .spacing(12)
        .align_y(Vertical::Center),
        text(
            (0..app.usb_count)
                .map(|i| format!(
                    "{}: {}",
                    if i == 0 {
                        "Faderust Cable".to_string()
                    } else {
                        format!("Faderust Cable {}", i + 1)
                    },
                    if i == 0 {
                        "verbunden"
                    } else {
                        "verbinde mit Windows …"
                    }
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
        .size(12)
        .color(MUTED),
        h3("Stream Deck"),
        row![
            text("Plugin 0.3.2 installiert, aktuell.")
                .size(13)
                .color(MUTED),
            space().width(Length::Fill),
            btn("Plugin aktualisieren", close(), false),
        ]
        .align_y(Vertical::Center),
        h3("Selbsttest"),
        row![
            text("Prüft Geräte und Engine.").size(13).color(MUTED),
            space().width(Length::Fill),
            btn("Starten", close(), false),
            btn("Protokoll", close(), false),
        ]
        .spacing(8)
        .align_y(Vertical::Center),
    ]
    .spacing(10);
    frame(
        "Einstellungen".into(),
        720.0,
        true,
        body.into(),
        row![
            btn("Abbrechen", close(), false),
            btn("Speichern", close(), true)
        ]
        .spacing(8)
        .into(),
    )
}
