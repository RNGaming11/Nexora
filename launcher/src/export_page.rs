//! Export… on Home's Instance row: the selected instance as a `.mrpack` or a `.zip`, with the
//! worlds the player switches on, written to the launcher's `exports/` folder
//! (`horizon_launcher::export`).
use super::*;
use horizon_launcher::export::{self, Format};
use std::path::PathBuf;

pub(super) struct ExportPanel {
    /// The instance it is for: picking another closes it.
    instance: String,
    format: Format,
    /// The instance's worlds (folder, name) and whether each goes in.
    worlds: Vec<(String, String, bool)>,
    busy: bool,
    /// The file written, or why not.
    done: Option<Result<(String, PathBuf), String>>,
}

impl Launcher {
    pub(super) fn toggle_export(&mut self) {
        if self.export.take().is_some() {
            return;
        }
        let Some(i) = self.selected_instance() else {
            return;
        };
        let worlds = horizon_launcher::worlds::list(&i.home)
            .into_iter()
            .map(|w| (w.folder, w.name, false))
            .collect();
        self.export = Some(ExportPanel {
            instance: i.id.clone(),
            format: Format::Mrpack,
            worlds,
            busy: false,
            done: None,
        });
    }

    fn run_export(&mut self, cx: &mut Context<Self>) {
        let Some(i) = self.selected_instance().cloned() else {
            return;
        };
        let Some(panel) = self.export.as_mut().filter(|p| !p.busy) else {
            return;
        };
        panel.busy = true;
        panel.done = None;
        let format = if i.java.is_some() { Format::Mrpack } else { panel.format };
        let java = i.java.clone();
        let worlds: Vec<String> = panel
            .worlds
            .iter()
            .filter(|w| w.2)
            .map(|w| w.0.clone())
            .collect();
        let task = cx.background_executor().spawn(async move {
            let out = export::free_path(&backend::data_dir().join("exports"), &i.name, format);
            let done = match java.as_ref() {
                Some(game) => export::export_java_mrpack(&i.home, &i.name, game, &worlds, &out),
                None => {
                    let linked = match format {
                        Format::Mrpack => export::linkable(&i.home, &export::from_modrinth),
                        Format::Zip => Vec::new(),
                    };
                    export::export(&i.home, &i.name, format, &worlds, &linked, &out)
                }
            };
            done.map(|done| {
                let linked = match done.linked {
                    0 => String::new(),
                    n => format!(", {n} listed by their Modrinth download"),
                };
                let file = done.path.file_name().unwrap_or_default().to_string_lossy();
                let summary = if java.is_some() {
                    format!("Created {file} for Modrinth: {} files.", done.copied)
                } else {
                    format!("Exported {file}: {} files{linked}.", done.copied)
                };
                (summary, done.path)
            })
        });
        cx.spawn(async move |this, cx| {
            let done = task.await;
            let _ = this.update(cx, |this, cx| {
                if let Some(panel) = &mut this.export {
                    panel.busy = false;
                    panel.done = Some(done);
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The panel under the Instance row, while it is open.
    pub(super) fn export_panel(&mut self, cx: &mut Context<Self>) -> Option<Div> {
        let t = self.theme;
        let selected = self.selected_instance()?.id.clone();
        if self.export.as_ref()?.instance != selected {
            self.export = None;
            return None;
        }
        let java_instance = self.selected_instance().is_some_and(|i| i.java.is_some());
        let panel = self.export.as_ref()?;
        let formats = if java_instance {
            vec![(Format::Mrpack, ".mrpack")]
        } else {
            vec![(Format::Mrpack, ".mrpack"), (Format::Zip, ".zip")]
        };
        let mut pick = div().flex().items_center().gap(px(6.));
        for (format, label) in formats {
            let on = panel.format == format;
            pick = pick.child(
                button(
                    SharedString::from(format!("export-{}", format.extension())),
                    label,
                    if on { Kind::Secondary } else { Kind::Ghost },
                    &t,
                )
                .when(on, |b| b.border_color(t.accent))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    if let Some(p) = &mut this.export {
                        p.format = format;
                    }
                    cx.notify();
                })),
            );
        }
        let about = match (java_instance, panel.format) {
            (true, Format::Mrpack) => {
                "Creates a Modrinth profile with this Minecraft version and loader. Its mods, \
                 settings, packs and selected worlds are bundled for local import."
            }
            (false, Format::Mrpack) => {
                "Modrinth's modpack format: resource packs from Modrinth are listed by their \
                 download, the rest is carried in it."
            }
            (_, Format::Zip) => "The game folder's own layout, zipped.",
        };
        let mut worlds = div().flex().flex_col().gap(px(6.));
        if panel.worlds.is_empty() {
            worlds = worlds.child(
                text(theme::CAPTION)
                    .text_color(t.muted)
                    .child("No worlds to put in."),
            );
        }
        for (n, (_, name, on)) in panel.worlds.iter().enumerate() {
            worlds = worlds.child(
                div()
                    .id(SharedString::from(format!("export-world-{n}")))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .child(switch(
                        SharedString::from(format!("export-world-switch-{n}")),
                        *on,
                        &t,
                    ))
                    .child(
                        text(theme::LABEL)
                            .text_color(t.ink_soft)
                            .child(name.clone()),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if let Some(w) = this.export.as_mut().and_then(|p| p.worlds.get_mut(n)) {
                            w.2 = !w.2;
                        }
                        cx.notify();
                    })),
            );
        }
        let go = button(\n            "export-run",\n            if java_instance { "Prepare for Modrinth" } else { "Export" },\n            Kind::Secondary,\n            &t,\n        );
        let go = match panel.busy {
            true => disabled(go),
            false => go.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.run_export(cx);
                cx.notify();
            })),
        };
        let result = match &panel.done {
            Some(Ok((said, path))) => {
                let show_path = path.clone();
                let open_path = path.clone();
                let open_in_modrinth = java_instance.then(|| {
                    button("export-open-modrinth", "Open in Modrinth", Kind::Secondary, &t)
                        .on_click(move |_, _, cx| cx.open_with_system(&open_path))
                });
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(text(theme::CAPTION).text_color(t.muted).child(said.clone()))
                        .child(
                            button("export-show", "Show in folder", Kind::Ghost, &t)
                                .on_click(move |_, _, cx| cx.reveal_path(&show_path)),
                        )
                        .children(open_in_modrinth),
                )
            }
            Some(Err(e)) => Some(div().child(notice(e.clone(), &t))),
            None => None,
        };
        Some(
            plate(&t)
                .flex()
                .flex_col()
                .gap(px(12.))
                .px(px(14.))
                .py(px(12.))
                .ml(px(40.))
                .child(caps(if java_instance { "Use Modrinth sign-in" } else { "Export to share" }).text_color(t.muted))
                .child(pick)
                .child(text(theme::CAPTION).text_color(t.muted).child(about))
                .child(caps("Worlds").text_color(t.muted))
                .child(worlds)
                .child(text(theme::CAPTION).text_color(t.muted).child(if java_instance {
                    "The package contains game files only, not your Microsoft account or login tokens. \
                     Modrinth imports it as a profile; sign in and press Play there."
                } else {
                    "Settings, packs and Horizon mods go in; logs, screenshots, backups and \
                     your account never do. Worlds go without their per-player files."
                }))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.))
                        .child(go)
                        .when(panel.busy, |d| {
                            d.child(text(theme::CAPTION).text_color(t.muted).child("Exporting…"))
                        }),
                )
                .children(result),
        )
    }
}
