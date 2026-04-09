param(
  [string]$OutputIco = "src-tauri/icons/icon.ico",
  [string]$OutputPng = "src-tauri/icons/icon-256.png",
  [string]$PreviewPng = "src-tauri/icons/icon-preview.png"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

function New-RoundedRectPath {
  param(
    [System.Drawing.RectangleF]$Rect,
    [float]$Radius
  )

  $diameter = $Radius * 2
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath

  $path.AddArc($Rect.X, $Rect.Y, $diameter, $diameter, 180, 90)
  $path.AddArc($Rect.Right - $diameter, $Rect.Y, $diameter, $diameter, 270, 90)
  $path.AddArc($Rect.Right - $diameter, $Rect.Bottom - $diameter, $diameter, $diameter, 0, 90)
  $path.AddArc($Rect.X, $Rect.Bottom - $diameter, $diameter, $diameter, 90, 90)
  $path.CloseFigure()

  return $path
}

function Fill-RoundedRect {
  param(
    [System.Drawing.Graphics]$Graphics,
    [System.Drawing.Brush]$Brush,
    [System.Drawing.RectangleF]$Rect,
    [float]$Radius
  )

  $path = New-RoundedRectPath -Rect $Rect -Radius $Radius
  try {
    $Graphics.FillPath($Brush, $path)
  }
  finally {
    $path.Dispose()
  }
}

function Save-PngAsIco {
  param(
    [string]$PngPath,
    [string]$IcoPath
  )

  $pngBytes = [System.IO.File]::ReadAllBytes((Resolve-Path $PngPath))
  $directory = Split-Path -Parent $IcoPath
  if ($directory) {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
  }

  $stream = [System.IO.File]::Open($IcoPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
  try {
    $writer = New-Object System.IO.BinaryWriter($stream)
    try {
      $writer.Write([uint16]0)
      $writer.Write([uint16]1)
      $writer.Write([uint16]1)
      $writer.Write([byte]0)
      $writer.Write([byte]0)
      $writer.Write([byte]0)
      $writer.Write([byte]0)
      $writer.Write([uint16]1)
      $writer.Write([uint16]32)
      $writer.Write([uint32]$pngBytes.Length)
      $writer.Write([uint32]22)
      $writer.Write($pngBytes)
    }
    finally {
      $writer.Dispose()
    }
  }
  finally {
    $stream.Dispose()
  }
}

$outputIcoPath = Join-Path (Get-Location) $OutputIco
$outputPngPath = Join-Path (Get-Location) $OutputPng
$previewPngPath = Join-Path (Get-Location) $PreviewPng

New-Item -ItemType Directory -Force -Path (Split-Path -Parent $outputPngPath) | Out-Null

$size = 256
$bitmap = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)

try {
  $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
  $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
  $graphics.Clear([System.Drawing.Color]::Transparent)

  $tileRect = [System.Drawing.RectangleF]::new(18, 18, 220, 220)
  $shadowRect = [System.Drawing.RectangleF]::new(18, 24, 220, 220)

  $shadowBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(34, 19, 29, 40))
  try {
    Fill-RoundedRect -Graphics $graphics -Brush $shadowBrush -Rect $shadowRect -Radius 54
  }
  finally {
    $shadowBrush.Dispose()
  }

  $basePath = New-RoundedRectPath -Rect $tileRect -Radius 54
  try {
    $baseGradient = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
      $tileRect,
      [System.Drawing.ColorTranslator]::FromHtml("#163654"),
      [System.Drawing.ColorTranslator]::FromHtml("#2F6C9C"),
      48
    )
    try {
      $blend = New-Object System.Drawing.Drawing2D.Blend
      $blend.Positions = [single[]](0.0, 0.45, 1.0)
      $blend.Factors = [single[]](0.0, 0.7, 1.0)
      $baseGradient.Blend = $blend
      $graphics.FillPath($baseGradient, $basePath)
    }
    finally {
      $baseGradient.Dispose()
    }

    $glowPath = New-Object System.Drawing.Drawing2D.GraphicsPath
    try {
      $glowPath.AddEllipse(-14, -4, 182, 140)
      $glowBrush = New-Object System.Drawing.Drawing2D.PathGradientBrush($glowPath)
      try {
        $glowBrush.CenterColor = [System.Drawing.Color]::FromArgb(92, 255, 250, 242)
        $glowBrush.SurroundColors = [System.Drawing.Color[]]@([System.Drawing.Color]::FromArgb(0, 255, 250, 242))
        $graphics.FillPath($glowBrush, $glowPath)
      }
      finally {
        $glowBrush.Dispose()
      }
    }
    finally {
      $glowPath.Dispose()
    }

    $borderPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(54, 255, 255, 255), 2)
    try {
      $graphics.DrawPath($borderPen, $basePath)
    }
    finally {
      $borderPen.Dispose()
    }
  }
  finally {
    $basePath.Dispose()
  }

  $outerDRect = [System.Drawing.RectangleF]::new(58, 54, 142, 148)
  $outerDBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(246, 255, 250, 242))
  try {
    Fill-RoundedRect -Graphics $graphics -Brush $outerDBrush -Rect $outerDRect -Radius 48
  }
  finally {
    $outerDBrush.Dispose()
  }

  $cutoutRect = [System.Drawing.RectangleF]::new(102, 84, 58, 88)
  $cutoutBrush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#234E77"))
  try {
    Fill-RoundedRect -Graphics $graphics -Brush $cutoutBrush -Rect $cutoutRect -Radius 24
  }
  finally {
    $cutoutBrush.Dispose()
  }

  $leftChannelBrush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#DCEAF6"))
  try {
    Fill-RoundedRect -Graphics $graphics -Brush $leftChannelBrush -Rect ([System.Drawing.RectangleF]::new(73, 68, 20, 120)) -Radius 10
  }
  finally {
    $leftChannelBrush.Dispose()
  }

  $bar1Brush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#7CC4F5"))
  $bar2Brush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#B8E2FF"))
  $bar3Brush = New-Object System.Drawing.SolidBrush([System.Drawing.ColorTranslator]::FromHtml("#F4C98A"))
  try {
    Fill-RoundedRect -Graphics $graphics -Brush $bar1Brush -Rect ([System.Drawing.RectangleF]::new(112, 124, 12, 34)) -Radius 6
    Fill-RoundedRect -Graphics $graphics -Brush $bar2Brush -Rect ([System.Drawing.RectangleF]::new(129, 108, 12, 50)) -Radius 6
    Fill-RoundedRect -Graphics $graphics -Brush $bar3Brush -Rect ([System.Drawing.RectangleF]::new(146, 94, 12, 64)) -Radius 6
  }
  finally {
    $bar1Brush.Dispose()
    $bar2Brush.Dispose()
    $bar3Brush.Dispose()
  }

  $focusPen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(102, 255, 255, 255), 6)
  try {
    $focusPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
    $focusPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
    $graphics.DrawArc($focusPen, 146, 44, 52, 52, 205, 86)
  }
  finally {
    $focusPen.Dispose()
  }

  $bitmap.Save($outputPngPath, [System.Drawing.Imaging.ImageFormat]::Png)
  $bitmap.Save($previewPngPath, [System.Drawing.Imaging.ImageFormat]::Png)
  Save-PngAsIco -PngPath $outputPngPath -IcoPath $outputIcoPath
}
finally {
  $graphics.Dispose()
  $bitmap.Dispose()
}

Write-Output "Generated $outputIcoPath"
Write-Output "Generated $outputPngPath"
Write-Output "Generated $previewPngPath"
