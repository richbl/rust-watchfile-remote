use serde_derive::Deserialize;
use ssh2::Session;
use std::fs::File;
use std::io::ErrorKind;
use std::io::{self, prelude::*};
use std::net::TcpStream;
use std::option::Option;
use std::path::Path;
use std::thread;
use std::time::Duration;

// -----------------------------------------------------------------------------
// Send file via SFTP
//
//   Arguments:
//     - destination server
//     - username
//     - password
//     - ssh private key path
//     - local file path
//     - remote file path
//   Returns:
//     - the success/failure of the file sent
//
fn send_file_via_sftp(
  remote_server: &str, username: &str, password: Option<&str>, ssh_key: &str, local_file_path: &str,
  remote_file_path: &str,
) -> io::Result<()> {
  //
  // Connect to the SSH server
  //
  let tcp = TcpStream::connect(format!("{}:22", remote_server))?;
  let mut session = Session::new()?;
  session.set_tcp_stream(tcp);
  session.handshake()?;

  //
  // Authenticate with username and password, else use existing SSH key specified on
  // the ssh_key path (note that userauth_pubkey_file() call is not dependent on ssh-agent)
  //
  if let Some(pwd) = password {
    session.userauth_password(username, pwd)?;
  } else {
    session.userauth_pubkey_file(username, None, ssh_key.as_ref(), Some(""))?;
  }

  //
  // Open an SFTP session and create remote file
  //
  let mut sftp = session.sftp()?;
  let mut remote_file = sftp.create(remote_file_path.as_ref())?;

  //
  // Read local file and write its content to the remote file
  //
  let mut local_file = File::open(local_file_path)?;
  let mut buffer = vec![0; 1024];

  loop {
    let bytes_read = local_file.read(&mut buffer)?;
    if bytes_read == 0 {
      break;
    }
    remote_file.write_all(&buffer[..bytes_read])?;
  }
  remote_file.close()?;

  //
  // We're done, so shutdown the SFTP session!
  //
  sftp.shutdown()?;
  session.disconnect(None, "", Some(""))?;

  Ok(())
}

// -----------------------------------------------------------------------------
// Structs to define config.toml file
//
#[derive(Debug, Default, Deserialize)]
struct Config {
  app: App,
  receiver: Receiver,
}

#[derive(Debug, Default, Deserialize)]
struct App {
  watchfile_name: String,
  watchfile_dir: String,
  sleep_interval: u64,
}

#[derive(Debug, Default, Deserialize)]
struct Receiver {
  username: String,
  server: String,
  password: String,
  ssh_key: String,
  dir: String,
}

// -----------------------------------------------------------------------------
// The main event
//
fn main() {
  //
  // Load configuration from TOML file
  //
  let config = match watchfilelib::load_toml_config::<Config>("watchfile-remote-sender.toml") {
    Ok(config) => config,
    Err(err) => {
      return eprintln!("{}", err);
    }
  };

  //
  // SFTP authentication can be managed in two ways:
  //
  // A. use SSH keys already shared between client and server (default)
  // B. send a password during SSH setup/SFTP transfer
  //
  // To use Option A: set receiver.password = "use_ssh_keys" in the TOML file
  // To use Option B: set receiver.password to the actual authentication password
  //
  let ssh_password: Option<&str> = match config.receiver.password.as_str() {
    "use_ssh_keys" => {
      let ssh_key_path = Path::new(&config.receiver.ssh_key);
      match ssh_key_path.try_exists() {
        Ok(exists) => {
          if !exists {
            return eprintln!("The configured SSH key ({}) does not exist", config.receiver.ssh_key);
          }
        }
        Err(err) => {
          return eprintln!("Error checking SSH key existence: {}", err);
        }
      }
      None
    }
    password => Some(password),
  };

  //
  // Parse and confirm the existence of watchfile_name found in watchfile_dir
  //
  let watchfile_path_local =
    Path::new(&config.app.watchfile_dir).join(&config.app.watchfile_name).display().to_string();

  match Path::new(&watchfile_path_local).try_exists() {
    Ok(exists) => {
      if !exists {
        return eprintln!("The local filepath {} does not exist", watchfile_path_local);
      }
    }
    Err(err) => {
      return eprintln!("Error checking local file existence: {}", err);
    }
  }

  let watchfile_path_receiver =
    (Path::new(&config.receiver.dir).join(&config.app.watchfile_name)).display().to_string();

  //
  // WARNING! infinite loop dead ahead
  //
  loop {
    //
    // Attempt to send the-watchfile to the receiver
    //
    match send_file_via_sftp(
      &config.receiver.server,
      &config.receiver.username,
      ssh_password.as_deref(),
      &config.receiver.ssh_key,
      &watchfile_path_local,
      &watchfile_path_receiver,
    ) {
      Ok(_) => {} // File successfully sent to the receiver!

      //
      // Bad news... the SFTP transaction failed, so match on the error generated
      //
      Err(e) => match e.kind() {
        ErrorKind::ConnectionRefused => eprintln!("Port 22 is closed on {}", config.receiver.server),
        ErrorKind::ConnectionReset => eprintln!("Connection reset by {}", config.receiver.server),
        ErrorKind::TimedOut => eprintln!("Transactions with {} timed out", config.receiver.server),
        ErrorKind::Unsupported => eprintln!("Transactions with {} are unsupported", config.receiver.server),
        _ => eprintln!("An undefined error occurred attempting to transact with {}: {}", config.receiver.server, e),
      },
    };

    //
    // Take a nap and dream of electric sheep
    //
    thread::sleep(Duration::from_secs(config.app.sleep_interval));
  }
}
