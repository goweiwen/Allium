/*
* cam_os_util_ioctl.h - Sigmastar
*
* Copyright (C) 2018 Sigmastar Technology Corp.
*
* Author: giggs.huang <giggs.huang@sigmastar.com.tw>
*
* This software is licensed under the terms of the GNU General Public
* License version 2, as published by the Free Software Foundation, and
* may be copied, distributed, and modified under those terms.
*
* This program is distributed in the hope that it will be useful,
* but WITHOUT ANY WARRANTY; without even the implied warranty of
* MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
* GNU General Public License for more details.
*
*/


#ifndef __CAM_OS_UTIL_IOCTL_H__
#define __CAM_OS_UTIL_IOCTL_H__

#ifdef __cplusplus
extern "C" {
#endif /* __cplusplus */

#define CAM_OS_IOC_NRBITS	8
#define CAM_OS_IOC_TYPEBITS	8

#define CAM_OS_IOC_SIZEBITS	14

#define CAM_OS_IOC_DIRBITS	2

#define CAM_OS_IOC_NRMASK	((1 << CAM_OS_IOC_NRBITS)-1)
#define CAM_OS_IOC_TYPEMASK	((1 << CAM_OS_IOC_TYPEBITS)-1)
#define CAM_OS_IOC_SIZEMASK	((1 << CAM_OS_IOC_SIZEBITS)-1)
#define CAM_OS_IOC_DIRMASK	((1 << CAM_OS_IOC_DIRBITS)-1)

#define CAM_OS_IOC_NRSHIFT      0
#define CAM_OS_IOC_TYPESHIFT    (CAM_OS_IOC_NRSHIFT+CAM_OS_IOC_NRBITS)
#define CAM_OS_IOC_SIZESHIFT    (CAM_OS_IOC_TYPESHIFT+CAM_OS_IOC_TYPEBITS)
#define CAM_OS_IOC_DIRSHIFT	    (CAM_OS_IOC_SIZESHIFT+CAM_OS_IOC_SIZEBITS)

#define CAM_OS_IOC_NONE	    0U
#define CAM_OS_IOC_WRITE    1U
#define CAM_OS_IOC_READ	    2U

#define CAM_OS_IOC(dir,type,nr,size) \
	(((dir)  << CAM_OS_IOC_DIRSHIFT) | \
	 ((type) << CAM_OS_IOC_TYPESHIFT) | \
	 ((nr)   << CAM_OS_IOC_NRSHIFT) | \
	 ((size) << CAM_OS_IOC_SIZESHIFT))

#define CAM_OS_IOC_TYPECHECK(t) (sizeof(t))

/* used to create numbers */
#define CAM_OS_IO(type,nr)              CAM_OS_IOC(CAM_OS_IOC_NONE,(type),(nr),0)
#define CAM_OS_IOR(type,nr,size)        CAM_OS_IOC(CAM_OS_IOC_READ,(type),(nr),(CAM_OS_IOC_TYPECHECK(size)))
#define CAM_OS_IOW(type,nr,size)        CAM_OS_IOC(CAM_OS_IOC_WRITE,(type),(nr),(CAM_OS_IOC_TYPECHECK(size)))
#define CAM_OS_IOWR(type,nr,size)       CAM_OS_IOC(CAM_OS_IOC_READ|CAM_OS_IOC_WRITE,(type),(nr),(CAM_OS_IOC_TYPECHECK(size)))
#define CAM_OS_IOR_BAD(type,nr,size)    CAM_OS_IOC(CAM_OS_IOC_READ,(type),(nr),sizeof(size))
#define CAM_OS_IOW_BAD(type,nr,size)    CAM_OS_IOC(CAM_OS_IOC_WRITE,(type),(nr),sizeof(size))
#define CAM_OS_IOWR_BAD(type,nr,size)   CAM_OS_IOC(CAM_OS_IOC_READ|CAM_OS_IOC_WRITE,(type),(nr),sizeof(size))

/* used to decode ioctl numbers.. */
#define CAM_OS_IOC_DIR(nr)		(((nr) >> CAM_OS_IOC_DIRSHIFT) & CAM_OS_IOC_DIRMASK)
#define CAM_OS_IOC_TYPE(nr)		(((nr) >> CAM_OS_IOC_TYPESHIFT) & CAM_OS_IOC_TYPEMASK)
#define CAM_OS_IOC_NR(nr)		(((nr) >> CAM_OS_IOC_NRSHIFT) & CAM_OS_IOC_NRMASK)
#define CAM_OS_IOC_SIZE(nr)		(((nr) >> CAM_OS_IOC_SIZESHIFT) & CAM_OS_IOC_SIZEMASK)

/* ...and for the drivers/sound files... */

#define CAM_OS_IOC_IN           (CAM_OS_IOC_WRITE << CAM_OS_IOC_DIRSHIFT)
#define CAM_OS_IOC_OUT          (CAM_OS_IOC_READ << CAM_OS_IOC_DIRSHIFT)
#define CAM_OS_IOC_INOUT        ((CAM_OS_IOC_WRITE|CAM_OS_IOC_READ) << CAM_OS_IOC_DIRSHIFT)
#define CAM_OS_IOCSIZE_MASK     (CAM_OS_IOC_SIZEMASK << CAM_OS_IOC_SIZESHIFT)
#define CAM_OS_IOCSIZE_SHIFT    (CAM_OS_IOC_SIZESHIFT)

#ifdef __cplusplus
}
#endif /* __cplusplus */

#endif //__CAM_OS_UTIL_IOCTL_H__
