use std::collections::VecDeque;

use anyhow::Result;
use common::command::Command;
use common::database::Database;
use common::display::Display;
use common::geom::Size;
use common::locale::{Locale, LocaleSettings};
use common::platform::{DefaultPlatform, Platform};
use common::resources::Resources;
use common::stylesheet::Stylesheet;
use common::view::View;
use log::trace;
use tokio::sync::mpsc::{self, Sender};
use type_map::TypeMap;

use crate::view::ScreenshotViewerView;

pub struct ScreenshotViewer<P: Platform> {
    platform: P,
    display: P::Display,
    res: Resources,
    view: ScreenshotViewerView,
}

impl ScreenshotViewer<DefaultPlatform> {
    pub fn new(mut platform: DefaultPlatform) -> Result<Self> {
        let mut display = platform.display()?;

        let mut map = TypeMap::new();
        map.insert(Database::new()?);
        map.insert(Stylesheet::load()?);
        map.insert(Locale::new(&LocaleSettings::load()?.lang));
        map.insert(Into::<Size>::into(display.size()));
        let res = Resources::new(map);

        {
            let styles = res.get::<Stylesheet>();
            display.clear(styles.ui.background_color)?;
        }
        display.save()?;

        let rect = display.bounding_box();
        let view = ScreenshotViewerView::new(rect, res.clone())?;

        Ok(Self {
            platform,
            display,
            res,
            view,
        })
    }

    pub async fn run_event_loop(&mut self) -> Result<()> {
        let (tx, mut rx) = mpsc::channel(100);

        loop {
            {
                let styles = self.res.get::<Stylesheet>();

                if self.view.should_draw() && self.view.draw(&mut self.display, &styles)? {
                    self.display.flush()?;
                }
            }

            tokio::select! {
                event = self.platform.poll() => {
                    trace!("event: {:?}", event);
                    let mut bubble = VecDeque::new();
                    self.view.handle_key_event(event, tx.clone(), &mut bubble).await?;
                }
            }

            while let Ok(cmd) = rx.try_recv() {
                self.handle_command(cmd, &tx).await?;
            }
        }
    }

    async fn handle_command(&mut self, command: Command, _sender: &Sender<Command>) -> Result<()> {
        trace!("command: {:?}", command);
        if let Command::Exit = command {
            std::process::exit(0)
        }
        Ok(())
    }
}
