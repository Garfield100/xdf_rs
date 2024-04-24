from pyxdf import load_xdf
import timeit
import os
from os.path import dirname, join

file_path = join(dirname(dirname(__file__)), 'example-files', 'tmp', 'xdf_001.xdf')
num_times = 10

print("Loading file: " + file_path)

time = timeit.timeit(f'load_xdf("{file_path}")',setup="from pyxdf import load_xdf", number=num_times)

print(f"Time to load {num_times} times: " + str(time) + " seconds")
print("Average time to load: " + str(time/num_times) + " seconds")
