import setuptools
from distutils.core import setup

setup(name='wavetk',
      version='0.4.1',
      description='Python bindings around the Rust Wave library',
      author='Thomas Hiscock',
      author_email='thomashk000@gmail.com',
      setup_requires=['wheel'],
      install_requires=['numpy'],
      packages=['wavetk'],
      package_data={'wavetk': ['*.so']},
      include_package_data=True)
