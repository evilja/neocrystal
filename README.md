## This repo will be discontinued when evilja/neocrystal-headless and evilja/neocrystal-client replaces it.

This app is written in Rust and is opiniated.

It supports mp3 only (you can change that easily i think, just make the constructor function of Songs see .flac files, though i'm not sure if mp3-duration library would work.)

It is meant for keyboard usage but there is a limited mouse support too, like clicking on songs or control buttons at the footer, and page buttons on the header.

It uses filename for titles, artist name for artist (obviously), and album for playlists. You can use album as album too, what it does is just adding it to the searchable string and displaying on the footer.

You can see or change keybinds at the top of crystal_manager.rs. They are in form of consts. Make sure to change consts and not match{} branch.

Song limit is theoretically usize::MAX - 1 but page indicator can get fucked. It does not expand when it becomes two digits or such. I'll add it though.

Current keybinds:

P O L M N U J F C V E G S R and arrow keys

P: Play the song at cursor location

O: Toggle extra mode ( for volume control )

L: Toggle loop

M/Right arrow: Seek +5 seconds

N/Left arrow: Seek -5 seconds

U/Up arrow: Move cursor / Volume up in extra mode

J/Down arrow: Move cursor / Volume down in extra mode

F: Toggle shuffle

C: Change artist name. You'll enter a string and press enter when you're done.

V: Change album/playlist. You'll enter a string and press enter when you're done.

E: Force next song. Next song algorithm guarantees that song will be the next track. This isn't cancellable.

G: Force full redraw. Useful when you get alsa underrun warnings on your terminal.

S: Stop (not actually, it just pauses.)

R: Resume




# neocrystal-headless
