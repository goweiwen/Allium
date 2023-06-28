#ifndef __MI_VDEC_EXTRA_H__
#define __MI_VDEC_EXTRA_H__

#include "mi_sys_datatype.h"
#include "mi_common_datatype.h"


typedef enum
{
    E_MI_VDEC_FRAME_TYPE_I = 0,
    E_MI_VDEC_FRAME_TYPE_P,
    E_MI_VDEC_FRAME_TYPE_B,
    E_MI_VDEC_FRAME_TYPE_OTHER,
    E_MI_VDEC_FRAME_TYPE_NUM
} mi_vdec_FrameType_e;


typedef enum
{
    E_MI_VDEC_DB_MODE_H264_H265   = 0x00,
    E_MI_VDEC_DB_MODE_VP9         = 0x01,
    E_MI_VDEC_DB_MODE_MAX
} mi_vdec_DbMode_e; // Decoder Buffer Mode


typedef struct {
    long counter;
} atomic64_t;

typedef struct mi_vdec_FrameInfoExt_s
{
    MI_PHY phyLumaAddr2bit;                   // physical address of Luma 2bit buffer
    MI_PHY phyChromaAddr2bit;                 // physical address of Chroma 2bit buffer
    MI_U8  u8LumaBitDepth;                    // Luma Frame bitdepth, support 8~10bits now
    MI_U8  u8ChromaBitDepth;                  // Chroma Frame bitdepth, support 8~10bits now
    MI_U16 u16Stride2bit;                     // stride of 2bits frame buffer
    MI_SYS_FrameTileMode_e eFrameTileMode;  // Frame tile mode
    MI_SYS_FrameScanMode_e eFrameScanMode;  // Frame scan mode
} mi_vdec_FrameInfoExt_t;


typedef struct
{
    MI_PHY phyLumaAddr;                           // frame buffer base + the start offset of current displayed luma data. Unit: byte.
    MI_PHY phyChromaAddr;                       // frame buffer base + the start offset of current displayed chroma data. Unit: byte.
    MI_U32 u32TimeStamp;                         // Time stamp(DTS, PTS) of current displayed frame. Unit: ms (todo: 90khz)
    MI_U32 u32IdL;                                    // low part of ID number
    MI_U32 u32IdH;                                   // high part of ID number
    MI_U16 u16Stride;                                   // stride
    MI_U16 u16Width;                                  // width
    MI_U16 u16Height;                                 // hight
    mi_vdec_FrameType_e eFrameType;    //< Frame type: I, P, B frame
    MI_SYS_FieldType_e eFieldType;         //< Field type: Top, Bottom, Both
} mi_vdec_FrameInfo_t;


typedef struct mi_vdec_DbInfo_s
{
    MI_BOOL bDbEnable;           // Decoder Buffer Enable
    MI_U8   u8DbSelect;          // Decoder Buffer select
    MI_BOOL bHMirror;
    MI_BOOL bVMirror;
    MI_BOOL bUncompressMode;
    MI_BOOL bBypassCodecMode;
    mi_vdec_DbMode_e eDbMode;        // Decoder Buffer mode
    MI_U16 u16StartX;
    MI_U16 u16StartY;
    MI_U16 u16HSize;
    MI_U16 u16VSize;
    MI_PHY phyDbBase;          // Decoder Buffer base addr
    MI_U16 u16DbStride;        // Decoder Buffer stride
    MI_U8  u8DbMiuSel;         // Decoder Buffer Miu select
    MI_PHY phyLbAddr;          // Lookaside buffer addr
    MI_U8  u8LbSize;           // Lookaside buffer size
    MI_U8  u8LbTableId;        // Lookaside buffer table Id
} mi_vdec_DbInfo_t; // Decoder Buffer Info

typedef struct mi_vdec_DispInfo_s
{
    MI_U16 u16HorSize;
    MI_U16 u16VerSize;
    MI_U32 u32FrameRate;
    MI_U8  u8Interlace;
    MI_U16 u16CropRight;
    MI_U16 u16CropLeft;
    MI_U16 u16CropBottom;
    MI_U16 u16CropTop;
    MI_U16 u16Stride;
} mi_vdec_DispInfo_t;

typedef struct mi_vdec_DispFrame_s
{
    mi_vdec_FrameInfo_t stFrmInfo;    //< frame information
    MI_U32 u32PriData;                         //< firmware private data
    MI_S32 s32Idx;                               //< index used by apiVDEC to manage VDEC_DispQ[][]
    mi_vdec_FrameInfoExt_t stFrmInfoExt;        // Frame Info Extend
    mi_vdec_DbInfo_t stDbInfo;
    mi_vdec_DispInfo_t stDispInfo;
    MI_SYS_PixelFormat_e ePixelFmt;
    atomic64_t u64FastChnId;
} mi_vdec_DispFrame_t;

typedef struct SS_Vdec_BufInfo {
    MI_SYS_BUF_HANDLE stVdecHandle;
    MI_SYS_BufInfo_t  stVdecBufInfo;
    MI_S32  s32Index;
    MI_BOOL bType;
}SS_Vdec_BufInfo;

#endif
