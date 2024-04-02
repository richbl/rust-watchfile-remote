use chrono::{DateTime, Local, Utc};
use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use serde_derive::Deserialize;
use std::fs;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

// -----------------------------------------------------------------------------
// Get the modification date of a file
//
//   Arguments:
//     - full pathname to the watchfile
//   Returns:
//     - the modification date in Unix epoch seconds
//
fn get_file_date(watchfile_path_local: &String) -> Option<u64> {
  if let Ok(metadata) = fs::metadata(watchfile_path_local) {
    if let Ok(modified) = metadata.modified() {
      if let Ok(epoch_secs) = modified.duration_since(SystemTime::UNIX_EPOCH) {
        return Some(epoch_secs.as_secs());
      }
    }
  };
  None
}

// -----------------------------------------------------------------------------
// Convert UNIX_EPOCH date into ISO datetime
//
//   Arguments:
//     - UNIX_EPOCH value
//   Returns:
//     - a formatted datetime string
//
fn unix_epoch_to_local_datetime_string(epoch_secs: u64) -> String {
  let utc_datetime = DateTime::<Utc>::from(UNIX_EPOCH + std::time::Duration::from_secs(epoch_secs));
  let local_datetime = utc_datetime.with_timezone(&Local);

  local_datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

// -----------------------------------------------------------------------------
// Send an email
//
//   Arguments:
//     - email subject
//     - email body
//     - TOML struct
//   Returns:
//     - success/failure of sending the email
//
fn send_email(email_subject: &str, email_body: &str, config: &Config) {
  let email = Message::builder()
    .from(format!("{} {}{}{}", config.email.from_email_name, "<", config.email.from_email_addr, ">").parse().unwrap())
    .reply_to(
      format!("{} {}{}{}", config.email.reply_to_email_name, "<", config.email.reply_to_email_addr, ">")
        .parse()
        .unwrap(),
    )
    .to(format!("{} {}{}{}", config.email.to_email_name, "<", config.email.to_email_addr, ">").parse().unwrap())
    .subject(email_subject.to_string())
    .header(ContentType::TEXT_PLAIN)
    .body(email_body.to_string())
    .unwrap();

  let creds = Credentials::new(config.email.account_username.to_string(), config.email.account_password.to_string());

  //
  // Open a remote TLS connection to gmail (port 587)
  //
  let mailer = SmtpTransport::starttls_relay(&config.email.smtp_host).unwrap().credentials(creds).build();

  //
  // Send the email already...
  //
  match mailer.send(&email) {
    Ok(_) => println!("Email sent successfully: {}", email_subject),
    Err(e) => {
      eprintln!("Could not send email: {e:?}");
    }
  }
}

// -----------------------------------------------------------------------------
// Structs to define config.toml file
//
#[derive(PartialEq, Default, Deserialize)]
struct Config {
  app: App,
  email: Email,
}

#[derive(PartialEq, Default, Deserialize)]
struct App {
  watchfile_dir: String,
  watchfile_name: String,
  sleep_interval_up: u64,
  sleep_interval_down: u64,
}

#[derive(PartialEq, Default, Deserialize)]
struct Email {
  from_email_name: String,
  from_email_addr: String,
  reply_to_email_name: String,
  reply_to_email_addr: String,
  to_email_name: String,
  to_email_addr: String,
  account_username: String,
  account_password: String,
  smtp_host: String,
}

// -----------------------------------------------------------------------------
// The main event
//
fn main() {
  //
  // Load configuration from TOML file
  //
  let config = match watchfilelib::load_toml_config::<Config>("watchfile-remote-receiver.toml") {
    Ok(config) => config,
    Err(err) => {
      return eprintln!("{}", err);
    }
  };

  //
  // Parse and confirm the existence of watchfile_name found in watchfile_dir
  //
  let watchfile_path_local =
    Path::new(&config.app.watchfile_dir).join(&config.app.watchfile_name).display().to_string();

  match Path::new(&watchfile_path_local).try_exists() {
    Ok(exists) => {
      if !exists {
        return eprintln!("The filepath {} does not exist", watchfile_path_local);
      }
    }
    Err(err) => {
      return eprintln!("Error checking local file existence: {}", err);
    }
  }

  //
  // Get initial timestamp from watchfile
  //
  //
  let mut initial_timestamp = match get_file_date(&watchfile_path_local) {
    Some(initial_timestamp) => initial_timestamp,
    None => {
      return eprintln!("Error: Unable to retrieve INITIAL timestamp from watchfile");
    }
  };

  //
  // Flag to identify if internet is up/down
  //
  let mut is_internet = true;

  //
  // WARNING! infinite loop dead ahead
  //
  loop {
    //
    // If the internet is up, all is well in the world, so sleep for a long time (sleep_interval_up),
    // otherwise bad things are afoot (no internet service!), so sleep less (sleep_interval_down) so
    // the status check happens with a higher frequency, so we can report back good news sooner...
    //
    let sleep_duration = if is_internet {
      Duration::from_secs(config.app.sleep_interval_up)
    } else {
      Duration::from_secs(config.app.sleep_interval_down)
    };

    //
    // Take a nap and dream of electric sheep
    //
    thread::sleep(sleep_duration);

    //
    // Get updated timestamp from file
    //
    let updated_timestamp = match get_file_date(&watchfile_path_local) {
      Some(updated_timestamp) => updated_timestamp,
      None => {
        return eprintln!("Unable to retrieve timestamp from watchfile ({})", &watchfile_path_local);
      }
    };

    //
    // If the timestamps are the same, then no updates received over this last interval... and that's not good!
    //
    if initial_timestamp == updated_timestamp {
      if is_internet {
        let email_body =
          format!("{}{}", "Home internet down as of ", unix_epoch_to_local_datetime_string(updated_timestamp));
        send_email("Home Internet Service is Down!", &email_body, &config);
        is_internet = false;
      }
    } else if !is_internet {
      let email_body =
        format!("{}{}", "Home internet is back up as of ", unix_epoch_to_local_datetime_string(updated_timestamp));
      send_email("Home Internet Service is Back Up!", &email_body, &config);
      is_internet = true;
    }

    //
    // Update timestamp to most recent and then go back to sleep
    //
    initial_timestamp = updated_timestamp;
  }
}
