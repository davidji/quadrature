
# Connector positions from board images

I'm trying to find the positions of the connectors
of a black pill board from an image I've scanned
of the board.

This probably isn't worth the effort of automating
for one board, but might be useful in future.

I've already used ImageJ/Fiji and the Hough circle
transform to get a list of points. Perhaps I could do
that in python as well.

    pip3 install --user sklearn
    pip3 install --user matplotlib

This script doesn't give good results. it certainly seems
to look worse that the image output by the ImageJ
suggests.

Lets go back to the beginning and try to improve things...

    pip3 install opencv-python 