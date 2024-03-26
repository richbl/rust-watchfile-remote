# Watchfile Remote [Rust Edition]

[![Rust Report Card](https://rust-reportcard.xuri.me/badge/github.com/richbl/rust-watchfile-remote)](https://rust-reportcard.xuri.me/report/github.com/richbl/rust-watchfile-remote)
![GitHub Release](https://img.shields.io/github/v/release/richbl/rust-watchfile-remote?include_prereleases&sort=semver)

**Watchfile Remote [Rust Edition]** is a simple pattern that configures both a sender (via the `watchfile-remote-sender` executable) and a receiver (`watchfile-remote-receiver`) to monitor a single file, passed at a given interval, between them for change (called "heartbeat monitoring"). Both of these executables are started once on each machine, and then run indefinitely, typically as a background process or service. If no change is identified after a certain period of time--that is the heartbeat is no longer detected--then an email is generated identifying loss of the heartbeat. Conversely, if the heartbeat is again detected, a follow-on email is generated indicating the resumption of the heartbeat.

<p align="center">
<picture><source media="(prefers-color-scheme: dark)" srcset="https://github.com/richbl/rust-watchfile-remote/assets/10182110/1f94d390-1c9a-4e6a-bc90-0c0e6b1446aa"><source media="(prefers-color-scheme: light)" srcset="https://github.com/richbl/rust-watchfile-remote/assets/10182110/1f94d390-1c9a-4e6a-bc90-0c0e6b1446aa"><img src="[https://github.com/richbl/rust-watchfile-remote/assets/10182110/1f94d390-1c9a-4e6a-bc90-0c0e6b1446aa](https://github.com/richbl/rust-watchfile-remote/assets/10182110/1f94d390-1c9a-4e6a-bc90-0c0e6b1446aa)" width=700></picture>
</p>

## Rationale

This project was really created to resolve a very simple use case: how to know when my home internet service goes down--and, more importantly, when it comes back up again--while I'm away from home. This is important because we live in a rural region of Western Washington, and our only internet service is available via DSL, which means above-ground phone lines: and phone lines and trees on windy days don't really behave well together.

So, this project is basically broken down into two parts:

- The sender, which is a local server on our home LAN. This server, as the sender component of this project, periodically (every five minutes) attempts to send a "heartbeat" in the form of a simple file up to a remote server (not on our home LAN). Nothing more. Pretty simple.
- The receiver, which is located on one of my remote web servers, will watch for any "heartbeat" updates to that watchfile at an interval of every (n) minutes (10 minutes is the default). If that watchfile has been modified in the intervening (n) minutes, then my home internet service is up and running. On the other hand, if that watchfile hasn't changed, it would suggest that my home internet service is down, so the receiver sends me an email with the bad news.

### Wait a Second!... You Already Wrote This as a Bash Script

Yep, that's right. [The first version of **Watchfile Remote** was written as a set of `bash` scripts](https://github.com/richbl/watchfile-remote). I wrote this project really as an exercise to understand how Rust can be written in such a way that it abstracts away external dependencies that `bash` scripts--by their very definition--rely upon. So, here's my take away:

- It takes more code logic in Rust to accomplish what I originally did in the `bash` scripts
- However... quite a few dependencies are removed with this project, since they've been natively (re)written in Rust to run across multiple platforms. A couple of examples:
    - No need to rely on an external `scp` (or `sftp`) utility command
    - No need to rely on an external mail program such as `mailx`
    - No need to rely on an external [GNU C Library (`glibc`)](https://www.gnu.org/software/libc/): [musl `libc`](https://musl.libc.org/) used instead (*)

> (*) On this last point, using musl was not an intentional design decision. Instead, it was a solution to the problem of how to deploy a Rust binary to an older device that was running an old version of `glibc` (the target server happens to be running Ubuntu 18.04, while I'm running 23.10).

All told, while this Rust implementation makes for a solution with fewer external dependencies, writing the original `bash` script was much quicker and simpler overall (with many fewer lines of code). So, if you're running on hardware with a known Unix-like environment, [you might find the original `bash` scripts more appropriate for your own use case](https://github.com/richbl/watchfile-remote). Otherwise, enjoy playing around with this 100% Pure Rust Edition of **Watchfile Remote**.

## Requirements

- Since this is a Rust project, you'll likely need to cross-compile both executables for your respective platforms (releases here are built using the Rust ["target triple"](https://doc.rust-lang.org/beta/rustc/platform-support.html) of `x86_64-unknown-linux-musl` to manage `glibc` dependencies)
- On the receiver, your email system needs to be configured so emails can be sent to you when appropriate. Since I use Gmail, [I needed to configure my gmail account with something called Google calls an "app password"](https://support.google.com/mail/answer/185833?hl=en)
- Recommended: since these executables are expected to communicate securely over the internet using the [SFTP protocol](https://en.wikipedia.org/wiki/SSH_File_Transfer_Protocol), it's highly recommended to establish secure credentials between sender and receiver using an SSH key exchange. Using encrypted keys is preferred, but not required: if it's not possible to use a key exchange, the `SFTP` process can be configured in these executables to pass a password between devices instead.

## Basic Usage

**Watchfile Remote [Rust Edition]** is broken into two separate components: the sender and the receiver. Each component is mapped to one of two executables:

- For the sender, use the `watchfile-remote-sender` executable
- For the receiver, use the `watchfile-remote-receiver` executable

### The Sender

The sender component is a local LAN computer (typically a server that's always on). Its role will be to periodically send a "heartbeat" file (called `the-watchfile`) to the receiver.

To configure the sender component:

1. Copy the `watchfile-remote-sender` executable to the machine in question
2. Edit the `watchfile-remote-sender.toml` file to accurately reflect your machine configuration. Note that this TOML file needs to be in the same folder as the `watchfile-remote-sender` executable
a. Importantly, determine how you want to use the `SFTP` command: either by passing a password directly into the executable, or by establishing an ssh key exchange (preferred). If you choose to use a password, edit the `password` key in the TOML file accordingly: **leaving it set to `use_ssh_keys` will do exactly what it says: use SSH keys instead of a password**
3. Run the executable in the background with the following syntax: `./watchfile-remote-sender &` (using the `nohup` command may be prepended to the command in the event the executable terminates after user logout)
a. Better still... set up a service (e.g., using `systemd` or equivalent) to run `watchfile-remote-sender` as a background service
4. Note that nothing appears to be happening. That's good: nothing should be happening, as this executable is simply looping through a 5-minute wait, and then quietly copying a file up to the receiving server
5. To confirm that the executable is running, on a Unix-like machine type `ps -ef | grep -i watch` and you should see the `watchfile-remote-sender` executable running

#### Editing the Sender TOML File

The `watchfile-remote-sender` executable is configured through the following values, editable via the `watchfile-remote-sender.toml` file:

    [app]
      watchfile_name = "the-watchfile"                   # the name of the file to be passed between machines
      watchfile_dir = "/home/user/rust-watchfile-remote" # the full path to this executable
      resend_attempts = 4                                # max attempts at SFTP resend before exiting
      resend_interval = 15                               # interval (in secs) between resend attempts
      sleep_interval = 300                               # interval (in secs) to send the watchfile to the receiver

    [receiver]
      username = "username_here"             # remote server account username
      server = "yourdomain.com"              # remote server domain name
      password = "use_ssh_keys"              # password OR keep as default to "use_ssh_keys"
      ssh_key = "/home/user/.ssh/id_ed25519" # full path (on local machine) to ssh certificate
      dir = "/home/user/watchfile-remote"    # full path to receiver executable on remote server

### The Receiver

The receiver component is a remote computer not on the local LAN (typically a remote web server, or similar machine to which you have access). Its role will be to periodically watch for modifications to a "heartbeat" file (called `the-watchfile`) sent by the sender.

To configure the receiver component:

1. Copy the `watchfile-remote-receiver` executable to the machine in question
2. Edit the `watchfile-remote-receiver.toml` file to accurately reflect both your email and machine configuration. Note that this TOML file needs to be in the same folder as the `watchfile-remote-receiver` executable
3. Run the executable in the background with the following syntax: `./watchfile-remote-receiver &` (note that using the `nohup` command may be prepended to the command in the event the executable terminates after user logout)
a. Better still... set up a service (e.g., using `systemd` or equivalent) to run `watchfile-remote-receiver` as a background service
4. Note that nothing appears to be happening. That's good: nothing should be happening, as this executable is simply looping through a 10-minute wait cycle, and then quietly checking a file called `the-watchfile` for any recent updates
5. To confirm that the executable is running, on a Unix-like machine type `ps -ef | grep -i watch` and you should see the `watchfile-remote-receiver` executable running

#### Editing the Receiver TOML File

The `watchfile-remote-receiver` executable is configured through the following values, editable via the `watchfile-remote-receiver.toml` file:

    [app]
      watchfile_name = "the-watchfile"                   # the name of the file to be passed between machines
      watchfile_dir = "/home/user/rust-watchfile-remote" # the full path to this executable
      sleep_interval_up = 600                            # interval (in secs) to check for the-watchfile updates when internet service is up
      sleep_interval_down = 420                          # interval (in secs) to check for the-watchfile updates when internet service is down

    [email]
      from_email_name = "Home Internet Service Watcher"     # sender (FROM:) text field
      from_email_addr = "username@yourdomain.com"           # sender (FROM:) address
      reply_to_email_name = "Home Internet Service Watcher" # REPLY: text field
      reply_to_email_addr = "username@yourdomain.com"       # REPLY: address
      to_email_name = "End User Name"                       # recipient (TO:) text field
      to_email_addr = "somebody@gmail.com"                  # recipient (TO:) address

      smtp_host = "smtp.gmail.com"                  # SMTP server address
      account_username = "username@yourdomain.com"  # email server account address
      account_password = "google_app_password_here" # email server account password

> Note that the `account_password` field assumes the use of a Google Gmail account that requires an "app password." If this is not the case (a different SMTP service is used), this field can be ignored. [Details of how to create a Google app password is available in this link.](https://knowledge.workspace.google.com/kb/how-to-create-app-passwords-000009237)

## Roadmap

- At the moment, there's not much of a roadmap to consider. In general, these are pretty simple executables doing some pretty basic stuff. But if you have any thoughts or ideas for improvement, send them my way.
