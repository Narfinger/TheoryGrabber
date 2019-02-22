TheoryGrabber
==========================
[![CircleCI](https://circleci.com/gh/Narfinger/TheoryGrabber.svg?style=svg)](https://circleci.com/gh/Narfinger/TheoryGrabber)

Grabs papers from arxiv and eccc, displayes them nicely and puts them into a google drive folder.

This needs cargo and rust. Also we need your google api details. Create a project under https://console.cloud.google.com/ with api access to drive. Go under Api&Services -> Credentials and download the json file. Put this into the directory under client_secret.json and you are done.

Compile & run with `cargo run` or `cargo run --release`
