use glob::{glob, Paths};
use std::{fs, io, io::Read, path::Path};
use toml::Value as tomlVal;

pub(crate) fn glob_files(
    cfg_file: &str,
    source: Option<&str>,
    verbosity: i8,
) -> Result<Paths, Box<dyn std::error::Error>> {
    let cfg_fh = fs::OpenOptions::new()
        .read(true)
        .write(false)
        .create(false)
        .open(cfg_file)?;
    let mut buf_reader = io::BufReader::new(cfg_fh);
    let mut contents = String::new();
    buf_reader.read_to_string(&mut contents)?;
    let toml_contents = contents.parse::<tomlVal>().unwrap();

    let source_glob = toml_contents
        .get("source-glob")
        .expect("Failed to find 'source-glob' heading in toml config")
        .as_str()
        .expect("Error taking source-glob value as string");

    let source = source.unwrap_or(source_glob);
    let glob_path = Path::new(&source);
    let glob_str = shellexpand::tilde(glob_path.to_str().unwrap());

    if verbosity > 0 {
        println!("Sourcing Markdown documents matching : {}", glob_str);
    }

    return Ok(glob(&glob_str).expect("Failed to read glob pattern"));
}

pub(crate) mod event {

    use rand::distributions::{Distribution, Uniform};
    use rand::rngs::ThreadRng;
    use tui::widgets::ListState;

    use std::io;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use termion::event::Key;
    use termion::input::TermRead;

    #[derive(Clone)]
    pub struct RandomSignal {
        distribution: Uniform<u64>,
        rng: ThreadRng,
    }

    impl RandomSignal {
        pub fn new(lower: u64, upper: u64) -> RandomSignal {
            RandomSignal {
                distribution: Uniform::new(lower, upper),
                rng: rand::thread_rng(),
            }
        }
    }

    impl Iterator for RandomSignal {
        type Item = u64;
        fn next(&mut self) -> Option<u64> {
            Some(self.distribution.sample(&mut self.rng))
        }
    }

    #[derive(Clone)]
    pub struct SinSignal {
        x: f64,
        interval: f64,
        period: f64,
        scale: f64,
    }

    impl SinSignal {
        pub fn new(interval: f64, period: f64, scale: f64) -> SinSignal {
            SinSignal {
                x: 0.0,
                interval,
                period,
                scale,
            }
        }
    }

    impl Iterator for SinSignal {
        type Item = (f64, f64);
        fn next(&mut self) -> Option<Self::Item> {
            let point = (self.x, (self.x * 1.0 / self.period).sin() * self.scale);
            self.x += self.interval;
            Some(point)
        }
    }

    pub struct TabsState<'a> {
        pub titles: Vec<&'a str>,
        pub index: usize,
    }

    impl<'a> TabsState<'a> {
        pub fn new(titles: Vec<&'a str>) -> TabsState {
            TabsState { titles, index: 0 }
        }
        pub fn next(&mut self) {
            self.index = (self.index + 1) % self.titles.len();
        }

        pub fn previous(&mut self) {
            if self.index > 0 {
                self.index -= 1;
            } else {
                self.index = self.titles.len() - 1;
            }
        }
    }

    pub struct StatefulList<T> {
        pub state: ListState,
        pub items: Vec<T>,
    }

    impl<T> StatefulList<T> {
        pub fn new() -> StatefulList<T> {
            StatefulList {
                state: ListState::default(),
                items: Vec::new(),
            }
        }

        pub fn with_items(items: Vec<T>) -> StatefulList<T> {
            StatefulList {
                state: ListState::default(),
                items,
            }
        }

        pub fn next(&mut self) {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }

        pub fn previous(&mut self) {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.items.len() - 1
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }

        pub fn unselect(&mut self) {
            self.state.select(None);
        }
    }

    pub enum Event<I> {
        Input(I),
        Tick,
    }

    /// A small event handler that wrap termion input and tick events. Each event
    /// type is handled in its own thread and returned to a common `Receiver`
    pub struct Events {
        rx: mpsc::Receiver<Event<Key>>,
        input_handle: thread::JoinHandle<()>,
        tick_handle: thread::JoinHandle<()>,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Config {
        pub tick_rate: Duration,
    }

    impl Default for Config {
        fn default() -> Config {
            Config {
                tick_rate: Duration::from_millis(250),
            }
        }
    }

    impl Events {
        pub fn new() -> Events {
            Events::with_config(Config::default())
        }

        pub fn with_config(config: Config) -> Events {
            let (tx, rx) = mpsc::channel();
            let input_handle = {
                let tx = tx.clone();
                thread::spawn(move || {
                    let stdin = io::stdin();
                    for evt in stdin.keys() {
                        if let Ok(key) = evt {
                            if let Err(err) = tx.send(Event::Input(key)) {
                                eprintln!("{}", err);
                                return;
                            }
                        }
                    }
                })
            };
            let tick_handle = {
                thread::spawn(move || loop {
                    if let Err(err) = tx.send(Event::Tick) {
                        eprintln!("{}", err);
                        break;
                    }
                    thread::sleep(config.tick_rate);
                })
            };
            Events {
                rx,
                input_handle,
                tick_handle,
            }
        }

        pub fn next(&self) -> Result<Event<Key>, mpsc::RecvError> {
            self.rx.recv()
        }
    }
}
