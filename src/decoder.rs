use std::{mem::MaybeUninit, ptr, slice};

use crate::{
    ffi::{
        vpx_codec_ctx, vpx_codec_dec_init_ver, vpx_codec_decode, vpx_codec_destroy,
        vpx_codec_err_t_VPX_CODEC_OK, vpx_codec_get_frame, vpx_codec_iter_t, vpx_codec_vp9_dx,
        vpx_img_fmt_VPX_IMG_FMT_I420, VPX_DECODER_ABI_VERSION,
    },
    Error, Frame,
};

pub struct Vp9Decoder {
    ctx: vpx_codec_ctx,
    iter: vpx_codec_iter_t,
}

impl Vp9Decoder {
    pub fn new() -> Self {
        let mut ctx = MaybeUninit::uninit();
        let cfg = MaybeUninit::zeroed();

        let ret = unsafe {
            vpx_codec_dec_init_ver(
                ctx.as_mut_ptr(),
                vpx_codec_vp9_dx(),
                cfg.as_ptr(),
                0,
                VPX_DECODER_ABI_VERSION as i32,
            )
        };

        assert_eq!(
            ret, vpx_codec_err_t_VPX_CODEC_OK,
            "failed to initialize decoder"
        );

        Self {
            ctx: unsafe { ctx.assume_init() },
            iter: ptr::null_mut(),
        }
    }

    pub fn decode(&mut self, data: &[u8]) -> Result<(), Error> {
        let ret = unsafe {
            vpx_codec_decode(
                &mut self.ctx,
                data.as_ptr(),
                data.len().try_into().unwrap(),
                ptr::null_mut(),
                0,
            )
        };

        self.iter = ptr::null_mut();

        if ret != vpx_codec_err_t_VPX_CODEC_OK {
            Err(Error(ret))
        } else {
            Ok(())
        }
    }

    pub fn next_frame(&mut self, frame: &mut Frame) -> Result<bool, Error> {
        let img = unsafe { vpx_codec_get_frame(&mut self.ctx, &mut self.iter) };
        if img.is_null() {
            return Ok(false);
        }

        let img = unsafe { &mut *img };

        if img.fmt != vpx_img_fmt_VPX_IMG_FMT_I420 {
            return Err(Error(600));
        }

        assert_eq!(
            frame.width(),
            img.d_w,
            "frame width does not match codec width ({} != {})",
            frame.width(),
            img.d_w
        );
        assert_eq!(
            frame.height(),
            img.d_h,
            "frame height does not match codec height ({} != {})",
            frame.height(),
            img.d_h
        );

        // Copy data into the Frame.
        unsafe {
            for y in 0..frame.height() {
                let y = y as usize;
                let width = frame.width() as usize;
                let height = frame.height() as usize;

                let y_plane =
                    slice::from_raw_parts(img.planes[0].add(y * img.stride[0] as usize), width);

                let range = (y * width)..((y + 1) * width);
                (&mut frame.y_plane[range]).copy_from_slice(y_plane);

                let range = (y * width / 2)..((y + 1) * (width / 2));

                if y < height / 2 {
                    let u_plane = slice::from_raw_parts(
                        img.planes[1].add(y * img.stride[1] as usize),
                        width / 2,
                    );
                    (&mut frame.u_plane[range.clone()]).copy_from_slice(u_plane);

                    let v_plane = slice::from_raw_parts(
                        img.planes[2].add(y * img.stride[2] as usize),
                        width / 2,
                    );
                    (&mut frame.v_plane[range]).copy_from_slice(v_plane);
                }
            }
        }

        Ok(true)
    }
}

impl Drop for Vp9Decoder {
    fn drop(&mut self) {
        unsafe {
            vpx_codec_destroy(&mut self.ctx);
        }
    }
}
