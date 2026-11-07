"""CFA4 #91 – RRDBNet (ESRGAN-generátor) PyTorch-ban, a Real-ESRGAN súlyainak betöltéséhez.

Az architektúra a publikált ESRGAN-leírás (Wang et al., 2018) szerinti: 23 RRDB-blokk, blokkonként
három sűrű maradékblokk, 0,2-es maradékskálázás, két legközelebbi-szomszédos nagyítás. A rétegek
neve megegyezik a Real-ESRGAN súlyfájljainak kulcsaival (`conv_first`, `body.N.rdbK.convM`,
`conv_body`, `conv_up1`, `conv_up2`, `conv_hr`, `conv_last`), hogy azok szigorú betöltéssel
(`strict=True`) olvashatók legyenek. Kettes nagyításnál a bemenet előbb 2×2-es blokkokba rendeződik
(pixel unshuffle), ahogy a `RealESRGAN_x2plus` várja.
"""
from __future__ import annotations

import torch
from torch import nn
from torch.nn import functional as F


class DenseBlock(nn.Module):
    def __init__(self, feat: int, grow: int) -> None:
        super().__init__()
        self.conv1 = nn.Conv2d(feat, grow, 3, 1, 1)
        self.conv2 = nn.Conv2d(feat + grow, grow, 3, 1, 1)
        self.conv3 = nn.Conv2d(feat + 2 * grow, grow, 3, 1, 1)
        self.conv4 = nn.Conv2d(feat + 3 * grow, grow, 3, 1, 1)
        self.conv5 = nn.Conv2d(feat + 4 * grow, feat, 3, 1, 1)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        a = F.leaky_relu(self.conv1(x), 0.2)
        b = F.leaky_relu(self.conv2(torch.cat((x, a), 1)), 0.2)
        c = F.leaky_relu(self.conv3(torch.cat((x, a, b), 1)), 0.2)
        d = F.leaky_relu(self.conv4(torch.cat((x, a, b, c), 1)), 0.2)
        e = self.conv5(torch.cat((x, a, b, c, d), 1))
        return e * 0.2 + x


class RRDB(nn.Module):
    def __init__(self, feat: int, grow: int) -> None:
        super().__init__()
        self.rdb1 = DenseBlock(feat, grow)
        self.rdb2 = DenseBlock(feat, grow)
        self.rdb3 = DenseBlock(feat, grow)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        return self.rdb3(self.rdb2(self.rdb1(x))) * 0.2 + x


class RRDBNet(nn.Module):
    def __init__(self, scale: int, feat: int = 64, blocks: int = 23, grow: int = 32) -> None:
        super().__init__()
        if scale not in (2, 4):
            raise ValueError(f"scale: 2 vagy 4 lehet, nem {scale!r}")
        self.scale = scale
        in_ch = 3 * 4 if scale == 2 else 3
        self.conv_first = nn.Conv2d(in_ch, feat, 3, 1, 1)
        self.body = nn.Sequential(*(RRDB(feat, grow) for _ in range(blocks)))
        self.conv_body = nn.Conv2d(feat, feat, 3, 1, 1)
        self.conv_up1 = nn.Conv2d(feat, feat, 3, 1, 1)
        self.conv_up2 = nn.Conv2d(feat, feat, 3, 1, 1)
        self.conv_hr = nn.Conv2d(feat, feat, 3, 1, 1)
        self.conv_last = nn.Conv2d(feat, 3, 3, 1, 1)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        if self.scale == 2:
            x = F.pixel_unshuffle(x, 2)
        feat = self.conv_first(x)
        feat = feat + self.conv_body(self.body(feat))
        feat = F.leaky_relu(self.conv_up1(F.interpolate(feat, scale_factor=2.0, mode="nearest")), 0.2)
        feat = F.leaky_relu(self.conv_up2(F.interpolate(feat, scale_factor=2.0, mode="nearest")), 0.2)
        return self.conv_last(F.leaky_relu(self.conv_hr(feat), 0.2))
