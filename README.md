This application is written in Rustlang.

You can compile it with "cargo build" (rust compiler)

Current keybindings:

U or KEY_UP -> go up OR volume up (special interaction mode)

J or KEY_DOWN -> go down OR volume down (special interaction mode)

KEY_RIGHT -> +5 seconds into the song

KEY_LEFT -> -5 seconds

f -> shuffle

p -> play the selected music

s -> pause

l -> loop mode

o -> special interaction mode

b -> blacklist song (unreachable by both auto next and p button)

r -> resume

h -> search (can't use h in a search) - you can change its keybind to something useless via the const in crystal_manager.rs

TODO

idk nothing

I'M AWARE

DiscordRPC does NOT update with SEEK functions.