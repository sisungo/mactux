#!/usr/bin/env python3

import sys
import os
import shutil
import mkrootfs_arch
import mkrootfs_alpine

project_root = os.path.dirname(os.path.realpath(__file__)) + '/../..'
project_target_dir = project_root + '/target'
rootfs_target_dir = project_target_dir + '/rootfs'

def usage():
    print('usage: ./mkrootfs [arch | alpine | clean]')
    exit(1)

def pre_build():
    if os.path.exists(rootfs_target_dir):
        print('error: there has already been an existing rootfs build, try running clean first?')
        exit(1)
    os.mkdir(rootfs_target_dir)

if len(sys.argv) <= 1:
    usage()

match sys.argv[1]:
    case 'arch':
        pre_build()
        mkrootfs_arch.build()
        exit(0)
    case 'alpine':
        pre_build()
        mkrootfs_alpine.build()
        exit(0)
    case 'clean':
        try:
            shutil.rmtree(rootfs_target_dir)
        except:
            print('warning: there is no need to run clean')
        exit(0)
