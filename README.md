# twedia

GUI to losslessly mess with media files

Useful for clipping or removing audio/video, instantly and without any loss in quality;
[demo!](https://www.youtube.com/watch?v=Embpni9rP1s)

# Install

## Using Nix

This will compile the program if
it cannot find it in your configured binary caches
```sh
nix profile install github:Elec3137/twedia
```

## Using flatpak

Download the `.flatpak` file from [releases](https://github.com/Elec3137/twedia/releases),
and install it with your app store of choice. (often just right click -> install)

Alternatively, here's a oneliner to fetch and install the latest release: `curl -L $(curl -s https://api.github.com/repos/Elec3137/twedia/releases/latest | grep '"browser_download_url":' | grep 'moe.pancake.twedia.flatpak' |  grep -o 'https://[^"]*') -o /tmp/moe.pancake.twedia.flatpak && flatpak install --user /tmp/moe.pancake.twedia.flatpak`
[source](https://gist.github.com/steinwaywhw/a4cd19cda655b8249d908261a62687f8?permalink_comment_id=5124113#gistcomment-5124113)


# Develop

The prefered way to develop is with `direnv` (`nix`) and `cargo`;
```sh
git clone https://github.com/Elec3137/twedia
cd twedia
direnv allow
cargo build # or `nix build`
cargo run # or `nix run`
```


# TODO

## main goals

1. obsolete the CLI for creating and writing the final output

2. test nix builds with other linux distributions

3. consider looking into macOS support

4. consider looking into windows support

## cosmic

1. fix window decorations (should be able to drag it, close by clicking on the X)

2. fix button and text styles

## *meta*

ordering is by relevance/interest

~50% probablility any one thing will actually happen
