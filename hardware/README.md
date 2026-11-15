# Hardware

> [!WARNING]
> The V3 hardware is still changing. The CAD exports and parts list here are a snapshot and may already be outdated.
> Onshape is the source of truth.

![V34 R1 assembly](../images/cad/v34-r1-assembly.png)

## CAD
The V3 design is on Onshape:
[Spin Coater V34 R1](https://cad.onshape.com/documents/92eb9cb52198894acd4b607c/v/db3ac1a34d2a62ffda81dc34/e/fb45d9e7be92a57d799034dd).
That link is pinned to the version these files were exported from.

[`cad/`](cad) has a STEP file for every part the team designed. Purchased parts (bearings, motor, belt, metal pulley)
aren't included; see the [bill of materials](bom.csv) for those. Most slicers (PrusaSlicer, OrcaSlicer, Bambu Studio)
can open STEP files directly.

| Drivetrain | Section view |
|---|---|
| ![Drivetrain](../images/cad/v34-r1-drivetrain.png) | ![Section](../images/cad/v34-r1-section.png) |

## Printed parts
From the V34 R1 export. TODO: confirm quantities, material and settings for each part.

| Part | File | Qty | Material | Notes |
|---|---|---|---|---|
| Bottom plate | [bottom-plate.step](cad/bottom-plate.step) | 1 | TODO | |
| Top plate | [top-plate.step](cad/top-plate.step) | 1 | TODO | |
| Left support | [left-support.step](cad/left-support.step) | 1 | TODO | |
| Right support | [right-support.step](cad/right-support.step) | 1 | TODO | |
| Top bearing mount | [top-bearing-mount.step](cad/top-bearing-mount.step) | 1 | TODO | |
| Center shaft | [center-shaft.step](cad/center-shaft.step) | 1 | TODO | |
| Slip ring | [slip-ring.step](cad/slip-ring.step) | 1 | TODO | |
| Plate carrier (chuck) | [plate-carrier.step](cad/plate-carrier.step) | 1 | TODO | |
| Shaft pulley (72T) | [shaft-pulley-72t.step](cad/shaft-pulley-72t.step) | 1 | TODO | |
| Motor mount | [motor-mount.step](cad/motor-mount.step) | 1 | TODO | |
| Left motor mount support | [left-motor-mount-support.step](cad/left-motor-mount-support.step) | 1 | TODO | |
| Right motor mount support | [right-motor-mount-support.step](cad/right-motor-mount-support.step) | 1 | TODO | |
| Inside vase | [inside-vase.step](cad/inside-vase.step) | 1 | TODO | |
| Outside vase | [outside-vase.step](cad/outside-vase.step) | 1 | TODO | |
| Left wall | [left-wall.step](cad/left-wall.step) | 1 | TODO | |
| Right wall | [right-wall.step](cad/right-wall.step) | 1 | TODO | |
| Back wall | [back-wall.step](cad/back-wall.step) | 1 | TODO | |
| Screen wall | [screen-wall.step](cad/screen-wall.step) | 1 | TODO | |
| Screen | [screen.step](cad/screen.step) | 1 | TODO | |
| 30 mm standoff | [30mm-standoff.step](cad/30mm-standoff.step) | 4 | TODO | |

### Print settings
TODO: printer, material, layer height, walls, infill, supports, and any parts that need special orientation.

## Bill of materials
[bom.csv](bom.csv) lists the purchased parts in the current build. It's approximate: quantities, prices and links are
still being filled in from the team's order sheet.
