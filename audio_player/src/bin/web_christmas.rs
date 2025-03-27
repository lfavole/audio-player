//! Play the Christmas songs from a specified URL.

use audio_player::web;
use compile_dotenv::compile_env;
web!(compile_env!("WEB_CHRISTMAS_URL"));
