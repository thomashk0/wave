import setuptools
from distutils.core import setup

setup(name='pywave',
      version='0.2.0',
      description='Python bindings around the Rust Wave library',
      author='Thomas Hiscock',
      author_email='thomashk000@gmail.com',
      setup_requires=['wheel'],
      install_requires=['numpy'],
      packages=['pywave'],
      package_data={'pywave': ['*.so']},
      include_package_data=True)
