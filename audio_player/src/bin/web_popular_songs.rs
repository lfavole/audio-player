//! Play the popular songs from a specified URL.

use audio_player::web;
use compile_dotenv::compile_env;
web!(compile_env!("WEB_POPULAR_SONGS_URL"), true);
