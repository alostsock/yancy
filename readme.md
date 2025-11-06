# yancy

<b>y</b>et <b>a</b>nother <b>n</b>egative <b>c</b>onversion thing<b>y</b>

Takes camera RAW images of film negatives, and converts them to positives.
Intended to be used when scanning color negatives with a digital camera.
Accepts multiple files (`-f`) or a single directory (`-d`) as input.

Executes the following steps for each image input:

1. Load a RAW image file (assumes sRGB color space, landscape orientation)

    a. Crop the image into halves if `--half-frame` is enabled
  
3. Determine edges of the film border, and color of the film backing
4. Crop the image
5. White balance the image using the film backing color
6. Invert colors
7. Stretch RGB histograms
8. Save the resulting image
