# Keep Score

The Keep Score is an integer from 0 to 100. `castellan-core` computes it. The same findings always produce the same number. There is no floating point in the formula.

A score is shown to a person only when confidence is at least 500 thousandths. A storage-only inspection therefore does not display a band, even if the storage pillar itself is fine. The UI must not invent a number such as 86.

## Pillars and weights

Weights are thousandths and sum to 1000.

| Pillar | Weight |
| --- | ---: |
| Storage | 220 |
| Stability | 180 |
| Performance | 180 |
| Security | 240 |
| Network | 180 |

A pillar that was not assessed is left out of the weighted average. It contributes 0 to confidence.

## What deducts

A finding deducts only when its status is Fail and its severity is not Info. Unknown never deducts. NotApplicable never deducts. Info never deducts, even on Fail.

Base points, before the finding weight and the curve:

| Severity | Points |
| --- | ---: |
| Info | 0 |
| Notice | 2 |
| Warning | 12 |
| Critical | 40 |

Each finding has a score weight stored as thousandths. The default weight is 1000, which means 1. The points applied are `base * weight_millis / 1000`, capped so a pillar cannot fall below 0.

Findings are applied in order of severity (Critical first), then by finding id. That order is part of the result.

## Curve

After a pillar has already lost 36 points, further Warnings deduct three quarters of their base (integer division) and further Notices deduct half. Critical is never reduced. The 36 points already lost stay lost.

Twenty Notices on one pillar at weight 1 sum to 38 points: eighteen Notices reach 36, and the nineteenth and twentieth deduct 1 each.

## Vetoes

A failed finding may carry a pillar ceiling, an overall ceiling, or both. Ceilings are 0 to 100. The tightest ceiling wins. A ceiling is applied after point deductions. A veto does not add extra points of its own.

## Overall score

For each assessed pillar, `pillar_score = min(100 - penalty, pillar_ceiling)` when a ceiling exists.

```text
weighted_sum = sum(pillar_score * weight)
weight_sum   = sum(weight)          # assessed pillars only
keep         = (weighted_sum + weight_sum / 2) / weight_sum
```

The added `weight_sum / 2` rounds half up. If an overall ceiling is set, the keep is then `min(keep, overall_ceiling)`.

If nothing was assessed, the keep is absent and confidence is 0. If every pillar was assessed and nothing failed, the keep is 100, band Fortified, confidence 1000.

## Confidence

Each assessed pillar's confidence is `decisive * 1000 / (decisive + unknown)`, where Pass, Fail, and NotApplicable are decisive. An assessed pillar with no findings has confidence 1000.

```text
confidence_millis = sum(pillar_confidence_millis * weight) / 1000
```

The divisor is 1000, the full weight table, not the sum of assessed weights. One assessed pillar cannot produce confidence 1000.

`display_band` returns no band when `confidence_millis < 500`.

## Bands

| Score | Band |
| --- | --- |
| 90–100 | Fortified |
| 75–89 | Sound |
| 50–74 | Needs attention |
| 0–49 | At risk |

## Golden example

All five pillars assessed. `STORAGE-003` is a failed Warning (12 points, storage becomes 88). `SEC-004` is a failed Critical (40 points) with a pillar ceiling of 40, so security becomes `min(60, 40) = 40`. The other pillars stay at 100.

```text
88*220 + 100*180 + 100*180 + 40*240 + 100*180 = 82960
weight_sum = 1000
keep = (82960 + 500) / 1000 = 83
```

83 is Sound. The security ceiling does not pull the whole keep down to 40, because the veto was a pillar ceiling.

## Storage volume finding

`STORAGE-001` uses free space as basis points of the volume (0–10000):

| Free space | Severity | Status |
| --- | --- | --- |
| at least 15% | Info | Pass |
| at least 10% and below 15% | Notice | Fail |
| at least 5% and below 10% | Warning | Fail |
| below 5% | Critical | Fail |

5% exactly is a Warning, because the test is `free_bp >= 500`. Only Storage is assessed, so the display band is absent. Source for caring about free space: [Manage drive space with Storage Sense](https://support.microsoft.com/en-us/windows/manage-drive-space-with-storage-sense-654f6ada-7bfc-45e5-966b-e24aded96ad5). The 15/10/5 bands are the product thresholds, not a quote from that page.
