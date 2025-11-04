# yancy

<b>y</b>et <b>a</b>nother <b>n</b>egative <b>c</b>onversion thing<b>y</b>

Takes camera RAW images of film negatives, and converts them to positives.
Intended to be used when scanning color negatives with a digital camera.
Accepts multiple files (-f) or a single directory (-d) as input.

Executes the following steps for each image input:

1. Load the RAW image file (assumes sRGB color space, landscape orientation)
2. Determine edges of the film border, and color of the film backing
3. Crop the image
4. White balance the image using the film backing color
5. Invert colors
6. Stretch RGB histograms
7. Save the resulting image
