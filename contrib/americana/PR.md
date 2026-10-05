Draft pull request for openstreetmap-americana. Apply with
`git am contrib/americana/*.patch` on upstream `main`.

---

## Add shields for German Autobahns and the Asian Highway Network

Two networks with many route relations currently fall back to the generic
text shield:

| network | route relations (taginfo) | sign |
|---|---|---|
| `BAB` | 190 | [Zeichen 405](https://commons.wikimedia.org/wiki/File:Zeichen_405_-_Nummernschild_f%C3%BCr_Autobahnen,_StVO_1992.svg): white on a blue horizontal hexagon |
| `AH` | 32 | Intergovernmental Agreement on the Asian Highway Network, [annex III](https://treaties.un.org/doc/source/RecentTexts/XI_B_34_E.pdf): a rectangular sign with "AH" and the route number, white or black inscription; most member states sign it white on blue |

Both use existing helpers and the standard palette; no new artwork:

```js
shields["AH"] = roundedRectShield(Color.shields.blue, Color.shields.white);
shields["BAB"] = hexagonHorizontalShield(30, Color.shields.blue, Color.shields.white);
```

`AH` sits with `e-road` as the Asian counterpart of the European E-road
shield; `BAB` sits with `DE:national` under Germany.

### Design notes

- `ref` is drawn verbatim, as for other networks: `BAB` routes are tagged
  `ref=A 48` ([wiki](https://wiki.openstreetmap.org/wiki/DE:Germany/Autobahn)),
  so the shield reads "A 48" although Zeichen 405 shows only "48".
  `DE:national` already draws "B 1" the same way. Dropping the prefix would
  need a new ref transformation in shieldlib and seems better discussed
  separately.
- Text sizes stay within the guide's range: 11.4–13.4 px for "A 1"…"A 115",
  11.0–14 px for "AH1"…"AH150"; widths stay within the generic 34 px limit.

### Testing

- `npm run shields` adds exactly `AH` and `BAB`; every other network and the
  global options are unchanged.
- The shield test gallery shows both with refs of 1–5 characters (add
  `"BAB"` and `"AH"` to the `networks` list in `src/shieldtest.js`).

![BAB and AH next to e-road and DE:national](https://github.com/luofang34/roadshield/raw/main/docs/images/extensions.svg)

The images were rendered by [roadshield](https://github.com/luofang34/roadshield),
a Rust port of shieldlib; its oracle sweep compares these networks with
this branch's `shieldtest.html` in Chrome at 1x and 2x and finds no
differences beyond anti-aliasing.
