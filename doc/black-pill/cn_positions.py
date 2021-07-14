
import csv
import numpy as np

from sklearn.cluster import DBSCAN
from sklearn import metrics
from sklearn.datasets.samples_generator import make_blobs
from sklearn.preprocessing import StandardScaler

import cv2



MM_PER_INCH = 25.4
DPI = 600.0
# resolution in dots per millimeter: the scan was in DPI
MM_PER_PIXEL = MM_PER_INCH/DPI

def extract_holes_from_image():
    image = cv2.imread('doc/black-pill/blackpill.png')
    output = image.copy()
    gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
    circles = cv2.HoughCircles(gray, cv2.HOUGH_GRADIENT, 1.2, 50)
    print(circles)
    return np.array([[x*MM_PER_PIXEL,y*MM_PER_PIXEL] for (x,y,_) in circles[0]])

def read_holes_from_csv():
    holes = []
    sizes = set()
    with open('doc/black-pill/blackpill-cn-positions.csv', 'r') as csvfile:
        reader = csv.reader(csvfile)
        for row in reader:
            (_, x_pixels, y_pixels, r_pixels, _, _, _, _, _) = row
            try:
                coordinates = [float(x_pixels)*MM_PER_PIXEL, float(y_pixels)*MM_PER_PIXEL]
                radius = float(r_pixels)*MM_PER_PIXEL
                # item = (coordinates, radius) 
                item = coordinates
                holes.append(item)
            except ValueError:
                pass
    return np.array(holes)

# holes = read_holes_from_csv()


# X = read_holes_from_csv()
X = extract_holes_from_image()

db = DBSCAN(eps=0.2, min_samples=1).fit(X)
core_samples_mask = np.zeros_like(db.labels_, dtype=bool)
core_samples_mask[db.core_sample_indices_] = True
labels = db.labels_

# Number of clusters in labels, ignoring noise if present.
n_clusters_ = len(set(labels)) - (1 if -1 in labels else 0)
n_noise_ = list(labels).count(-1)

print('Estimated number of clusters: %d' % n_clusters_)
print('Estimated number of noise points: %d' % n_noise_)
# print("Silhouette Coefficient: %0.3f" % metrics.silhouette_score(X, labels))

# #############################################################################
# Plot result
import matplotlib.pyplot as plt

# Black removed and is used for noise instead.
unique_labels = set(labels)
colors = [plt.cm.Spectral(each)
          for each in np.linspace(0, 1, len(unique_labels))]
for k, col in zip(unique_labels, colors):
    if k == -1:
        # Black used for noise.
        col = [0, 0, 0, 1]

    class_member_mask = (labels == k)

    xy = X[class_member_mask & core_samples_mask]
    plt.plot(xy[:, 0], xy[:, 1], 'o', markerfacecolor=tuple(col),
             markeredgecolor='k', markersize=14)

    xy = X[class_member_mask & ~core_samples_mask]
    plt.plot(xy[:, 0], xy[:, 1], 'o', markerfacecolor=tuple(col),
             markeredgecolor='k', markersize=6)

plt.title('Estimated number of clusters: %d' % n_clusters_)
plt.show()

