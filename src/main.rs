use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::{env, fs};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use headless_chrome::{Browser, LaunchOptions, Tab};

const FORECAST_FULL_PATH: &str = "forecast_full.txt";
const FORECAST_ABBREVIATED_PATH: &str = "forecast_abbreviated.txt";

fn main() -> Result<(), Box<dyn Error>> {
    loop {
        let browser = Browser::new(LaunchOptions {
            headless: true, // For debugging
            ..Default::default()
        })?;

        let tab = browser.wait_for_initial_tab()?;

        // Navigate to higher summits forecast
        tab.navigate_to("https://www.mountwashington.org/experience-the-weather/higher-summit-forecast.aspx")?;

        // Wait for network/javascript/dom to load forecast
        tab.wait_for_element("div#SummitOutlook")?.click()?;

        // Fetch forecasts
        let full_forecast = fetch_higher_summits_forecast(&tab).expect("Could not fetch forecast");
        let abbreviated_forecast = fetch_abbreviated_forecast(&tab)?;

        // Setup if no files exist and it's the first run
        setup(&full_forecast, &abbreviated_forecast);

        // Only update files + print out forecasts if they have changed
        if hash(&full_forecast) != hash(&fs::read_to_string(FORECAST_FULL_PATH)?) {
            persist_forecast(&full_forecast, FORECAST_FULL_PATH)?;
        }

        if hash(&abbreviated_forecast) != hash(&fs::read_to_string(FORECAST_ABBREVIATED_PATH)?) {
            persist_forecast(&abbreviated_forecast, FORECAST_ABBREVIATED_PATH)?;

            // TODO: Add CLI toggle flag for actually sending sat messages
            //send_message_to_inreach(tab, abbreviated_forecast).expect("Could not send message to inreach");
        }

        // Check again for updates in 1 minute
        sleep(Duration::from_secs(60));
    }
}

fn setup(full_forecast: &String, abbreviated_forecast: &String) {
    // Base condition, no forecasts exists. TODO: Cleanup
    if !Path::new(FORECAST_FULL_PATH).exists() {
        persist_forecast(&full_forecast, FORECAST_FULL_PATH).expect("Could not write full forecast");
    }
    if !Path::new(FORECAST_ABBREVIATED_PATH).exists() {
        persist_forecast(&abbreviated_forecast, FORECAST_ABBREVIATED_PATH).expect("Could not write abbreviated forecast");
    }
}

// This full, detailed summary is often around ~2k characters
fn fetch_higher_summits_forecast(tab: &Arc<Tab>) -> Result<String, Box<dyn Error>> {
    let elem = tab.wait_for_element("#SummitOutlook > p")?;

    // Snag larger forecast
    let forecast = elem.get_inner_text().unwrap();

    Ok(forecast)
}

// This abbreviated forecast is typically around ~700 characters
fn fetch_abbreviated_forecast(tab: &Arc<Tab>) -> Result<String, Box<dyn Error>> {
    // Collect information for all days into one string
    let abbreviated_forecast = tab.wait_for_elements("#SummitOutlook > div")?
        .into_iter()
        .map(|e| e.get_inner_text().unwrap().replace("\n", ""))
        .collect::<Vec<_>>()
        .join(" ");

    Ok(abbreviated_forecast)
}

fn persist_forecast(forecast: &String, filename: &str) -> Result<(), Box<dyn Error>> {
    // Dump the forecast to console
    println!("{}", forecast);

    // Write forecast to disk
    let mut output = File::create(filename)?;
    write!(output, "{}", forecast)?;
    Ok(())
}

fn send_message_to_inreach(tab: Arc<Tab>, forecast: String) -> Result<(), Box<dyn Error>> {
    // Navigate to a verified URL
    tab.navigate_to(&env::var("GARMIN_MESSAGE_REPLY_URL").expect("Could not fetch value for envvar GARMIN_MESSAGE_REPLY_URL"))?;

    // Activate the textarea
    tab.wait_for_element("#ReplyMessage")?.click()?;
    tab.press_key("Enter")?;

    // Split into SMS message size
    let sub_string = split_string_into_sms_message_lengths(&forecast);

    println!("Safe: {:?}", sub_string.len());
    println!("Safe: {:?}", sub_string);

    tab.type_str(forecast.as_str())?;

    // Click send
    //tab.wait_for_element("#sendBtn")?.click()?;

    Ok(())
}


fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn split_string_into_sms_message_lengths(forecast: &String) -> Vec<String> {
    const TEXT_MESSAGE_LENGTH: usize = 160;
    let mut chars = forecast.chars();
    let sub_string = (0..)
        .map(|_| chars.by_ref().take(TEXT_MESSAGE_LENGTH).collect::<String>())
        .take_while(|s| !s.is_empty())
        .collect::<Vec<_>>();
    sub_string
}

