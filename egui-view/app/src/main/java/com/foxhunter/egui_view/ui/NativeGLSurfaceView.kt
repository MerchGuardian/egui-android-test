package com.foxhunter.egui_view.ui

import android.content.Context
import android.opengl.GLSurfaceView
import android.util.Log
import android.view.SurfaceHolder
import javax.microedition.khronos.egl.EGL10
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.egl.EGLContext
import javax.microedition.khronos.egl.EGLDisplay
import javax.microedition.khronos.opengles.GL10

private const val EGL_CONTEXT_CLIENT_VERSION = 0x3098
private const val glVersion = 3.0

class NativeGLSurfaceView(context: Context?) : GLSurfaceView(context) {
    private val renderer: NativeGLRenderer

    val nativeSurface: Long

    init {
        setEGLContextFactory(object: EGLContextFactory {
            override fun createContext(egl: EGL10?, display: EGLDisplay?, eglConfig: EGLConfig?): EGLContext {
                Log.i("egui_view", "createContext: egl: $egl, display: $display, eglConfig: $eglConfig")

                val ctx = egl!!.eglCreateContext(
                    display,
                    eglConfig,
                    EGL10.EGL_NO_CONTEXT,
                    intArrayOf(EGL_CONTEXT_CLIENT_VERSION, glVersion.toInt(), EGL10.EGL_NONE))
                return ctx
            }

            override fun destroyContext(egl: EGL10?, display: EGLDisplay?, context: EGLContext?) {
                Log.i("egui_view", "destroyContext: egl: $egl, display: $display, context: $context")
                egl!!.eglDestroyContext(display, context)
            }
        })

        renderer = NativeGLRenderer(this)

        setRenderer(renderer)

        nativeSurface = createNativeSurface0()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        super.surfaceDestroyed(holder)
        Log.i("egui_view", "surfaceDestroyed: holder: $holder")
        destroyNativeSurface0(nativeSurface)
    }

    companion object {
        init {
            System.loadLibrary("native_gl_surface")
        }

        @JvmStatic
        private external fun createNativeSurface0(): Long

        @JvmStatic
        private external fun destroyNativeSurface0(handle: Long)
    }
}

class NativeGLRenderer(val glSurfaceView: NativeGLSurfaceView) : GLSurfaceView.Renderer {
    override fun onDrawFrame(gl: GL10) {
        onDrawFrame0(glSurfaceView.nativeSurface)
    }

    override fun onSurfaceCreated(gl: GL10?, conig: EGLConfig?) {
        Log.i("egui_view", "onSurfaceCreated")
        onSurfaceCreated0(glSurfaceView.nativeSurface)
    }

    override fun onSurfaceChanged(gl: GL10, width: Int, height: Int) {
        Log.i("egui_view", "onSurfaceChanged: width: $width, height: $height")
        onSurfaceChanged0(glSurfaceView.nativeSurface, width, height)
    }

    companion object {
        @JvmStatic
        private external fun onDrawFrame0(nativeSurface: Long)
        @JvmStatic
        private external fun onSurfaceCreated0(nativeSurface: Long)
        @JvmStatic
        private external fun onSurfaceChanged0(nativeSurface: Long, width: Int, height: Int)
    }
}
