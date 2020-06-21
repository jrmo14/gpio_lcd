use parking_lot::Mutex;
use std::collections::BinaryHeap;
use std::sync::atomic::Ordering::AcqRel;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::thread::{sleep, JoinHandle};
use std::time::{Duration, Instant};

use crate::lcd::LcdDriver;
use std::cmp::Ordering;

pub struct ThreadedLcd {
    lcd_driver: Arc<Mutex<LcdDriver>>,
    job_list: Arc<Mutex<Vec<Job>>>,
    execution_thread: JoinHandle<()>,
}

#[derive(Debug, Clone)]
pub struct Job {
    text: String,
    row: u8,
    index: i32,
    rate: Option<Duration>,
    last_run: Option<Instant>,
}

impl ThreadedLcd {
    pub fn new(
        cols: u8,
        rows: u8,
        chip_str: &str,
        four_bit_mode: bool,
        rs: u8,
        rw: u8,
        enable: u8,
        d0: u8,
        d1: u8,
        d2: u8,
        d3: u8,
        d4: u8,
        d5: u8,
        d6: u8,
        d7: u8,
    ) -> Self {
        ThreadedLcd::with_driver(
            LcdDriver::new(
                cols,
                rows,
                chip_str,
                four_bit_mode,
                rs,
                rw,
                enable,
                d0,
                d1,
                d2,
                d3,
                d4,
                d5,
                d6,
                d7,
            )
            .unwrap(),
        )
    }

    pub fn with_driver(lcd: LcdDriver) -> Self {
        // Interesting idea would be to make this a hashmap based on the interval between jobs and execute on that key....
        let job_list = Arc::new(Mutex::new(Vec::<Job>::new()));
        let lcd_driver = Arc::new(Mutex::new(lcd));
        let thread_job_list = Arc::clone(&job_list);
        let thread_lcd_driver = Arc::clone(&lcd_driver);
        let execution_thread = thread::spawn(move || loop {
            let mut job_list = thread_job_list.lock();
            match job_list.first_mut() {
                Some(job) => {
                    // Wait for delay, so we run on time
                    if job.last_run.is_some() {
                        let sleep_time = job
                            .rate
                            .unwrap()
                            .checked_sub(job.last_run.unwrap().elapsed());
                        if sleep_time.is_some() {
                            sleep(sleep_time.unwrap());
                        }
                    }
                    // Pass the cloned Arc to the lcd_driver
                    job.run(thread_lcd_driver.clone());
                    // Run the job
                    (*job).last_run = Some(Instant::now());
                    // Remove from queue if it's a one off
                    if job.rate.is_none() {
                        job_list.remove(0);
                    }
                    // Sort so we get the next one on top
                    job_list.sort();
                }
                // This is not ideal cuz busy wait
                None => {}
            }
        });
        ThreadedLcd {
            job_list,
            lcd_driver,
            execution_thread,
        }
    }
    pub fn add_job(&self, job: Job) {
        let mut job_list = self.job_list.lock();
        job_list.push(job);
        job_list.sort()
    }
    pub fn clear_jobs(&self) {
        println!("Trying to grab job list lock");
        self.job_list.lock().clear();
        println!("List cleared")
    }

    pub fn clear_row(&self, row: u8) {
        println!("Trying to clear {}", row);
        self.job_list.lock().retain(|job| job.row != row);
        println!("Adding clear job");
        self.add_job(Job::new("", row, None));
        println!("Cleared row {}", row);
    }
}

impl Job {
    pub fn new(text: &str, row: u8, rate: Option<Duration>) -> Self {
        Job {
            text: text.to_string(),
            row,
            index: 0,
            rate,
            last_run: None,
        }
    }

    pub fn run(&mut self, driver: Arc<Mutex<LcdDriver>>) {
        let driver = driver.lock();
        driver.set_cursor(self.row, 0);
        let formatted_string = if self.text.len() <= driver.get_cols() as usize {
            format!(
                "{: <width$}",
                self.text.clone(),
                width = driver.get_cols() as usize
            )
        } else if self.text.len() < (self.index + driver.get_cols() as i32) as usize {
            format!(
                "{: <width$}",
                self.text.clone().as_str()[self.index as usize..self.text.len()].to_string(),
                width = driver.get_cols() as usize,
            )
        } else if self.index < 0 {
            let slice = self.text.clone().as_str()
                [0..(self.index + driver.get_cols() as i32) as usize]
                .to_string();
            format!("{: >width$}", slice, width = driver.get_cols() as usize)
        } else {
            self.text.clone().as_str()
                [self.index as usize..(self.index + driver.get_cols() as i32) as usize]
                .to_string()
        };
        driver.print(formatted_string.as_str());
        self.index += 1;
        if self.index > self.text.len() as i32 {
            self.index = -((driver.get_cols() / 2) as i32);
        }
    }
}

impl Eq for Job {}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.last_run.eq(&other.last_run)
    }
}

/*
(0. Jobs that should have already been run) Might want to look into making this happen...
1. One off jobs
2. Un run scheduled jobs
3. Next job to be run on schedule
*/
impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.rate {
            Some(rate) => match other.rate {
                Some(other_rate) => match self.last_run {
                    Some(last_run) => match other.last_run {
                        Some(other_last_run) => {
                            let now = Instant::now();
                            let self_dif = rate.checked_sub(now - last_run);
                            let other_dif = other_rate.checked_sub(now - other_last_run);
                            match self_dif {
                                Some(self_dif) => match other_dif {
                                    Some(other_dif) => self_dif.cmp(&other_dif),
                                    None => Ordering::Greater,
                                },
                                None => match other_dif {
                                    Some(other_dif) => Ordering::Less,
                                    None => Ordering::Equal,
                                },
                            }
                        }
                        None => Ordering::Greater,
                    },
                    None => match other.last_run {
                        Some(other_last_run) => Ordering::Less,
                        None => rate.cmp(&other_rate),
                    },
                },
                None => Ordering::Greater,
            },
            None => match other.rate {
                Some(other_rate) => Ordering::Less,
                None => Ordering::Equal,
            },
        }
    }
}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

mod test {
    use crate::scheduler::*;
    use std::time::{Duration, Instant};

    #[test]
    fn job_sort_test() {
        let now = Instant::now();

        let mut ref_vec = vec![
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: None,
                last_run: None,
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(100)),
                last_run: None,
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(150)),
                last_run: Option::from(now - Duration::from_millis(111)),
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(250)),
                last_run: Option::from(now - Duration::from_millis(111)),
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(250)),
                last_run: Option::from(now - Duration::from_millis(100)),
            },
        ];

        let mut test_vec = vec![
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(250)),
                last_run: Option::from(now - Duration::from_millis(111)),
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(150)),
                last_run: Option::from(now - Duration::from_millis(111)),
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(100)),
                last_run: None,
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: Option::from(Duration::from_millis(250)),
                last_run: Option::from(now - Duration::from_millis(100)),
            },
            Job {
                text: "".to_string(),
                row: 0,
                index: 0,
                rate: None,
                last_run: None,
            },
        ];

        test_vec.sort();
        assert_eq!(ref_vec, test_vec)
    }
}
