# contributing pride icons to confy

thanks for wanting to add to the pride icon set! here's everything you need to know.

## what you're making

a 128x128 pixel art version of the confy C logo, colored with a pride flag's colors. the C is tilted and has a geometric panel layout, so you're filling in the panels by feel rather than painting flat stripes. eyeball it, it'll look better that way.

## getting started

grab `contributing/base.ase` and open it in aseprite. it has the outline and corner orbs already placed. all you need to do is fill in the panels with your flag's colors.

there's no strict rule for which color goes where. the logo is slanted, so just do what looks right to you. look at the existing icons in this folder for reference if you're unsure.

## file format

- **canvas size:** 128x128
- **format:** export as `.png` with a transparent background, at 500% scaling
- **also include:** your `.ase` file so others can edit it later
- **filename:** `[flag name].png` and `[flag name].ase`
  - keep it lowercase, use hyphens, no spaces
  - example: `bisexual.png`

> [!NOTE] `.piskel` files are accepted, just export the pngs as 5x size and clarify this in your pull request.

## submitting

open a PR with your files added to `branding/pride/`. give it a short description of which flag you made. that's it.

if you don't want to make one yourself but want to request a flag that isn't here yet, open an issue and tag it `pride icon request` and i'll get to it.

## what's not ok

- anything sexual, explicit, or unrelated to pride flags
- flags or symbols associated with hate groups or harassment campaigns (eg, any nazi/fascist imagery dressed up as a pride symbol, TERF (trans exclusionary radical feminist) symbols, etc)
- low effort submissions (ms paint scribbles, broken transparency, wrong canvas size, etc.)

all PRs go through review before merging, so use common sense.

## attribution

if you've made and submitted a PR, great! why not go ahead and edit branding/pride/artists.toml to add your attribution to the flag? additions are easy and look like this:

```toml
[foobar] # replace "foobar" with your flag name!

artists = [
    { name = "johndoe", link = "https://example.com" },
    { name = "janedoe", link = "https://example.com/jane" },
]

```

> [!NOTE] the text "foobar", "johndoe", and other fields are placeholders. replace them with your real info so people know who contributed! remove the # comments in the above example, they aren't needed and serve to just show you what fields are which.

## license

all assets in this directory are licensed under CC BY-SA 4.0. by submitting, you agree your contribution falls under the same license. the `.ase` source files are included intentionally so people can remix them for confy forks or personal use.
