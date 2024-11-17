package com.foxhunter.egui_view.ui

import android.content.Context
import android.opengl.GLSurfaceView
import android.util.Log
import javax.microedition.khronos.egl.EGL10
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.egl.EGLContext
import javax.microedition.khronos.egl.EGLDisplay
import javax.microedition.khronos.opengles.GL10

private const val EGL_CONTEXT_CLIENT_VERSION = 0x3098
private const val glVersion = 3.0

class NativeGLSurfaceView(context: Context?) : GLSurfaceView(context) {
    private val renderer: NativeGLRenderer
    private var nativeContext: Long = 0

    init {
        setEGLContextFactory(object: EGLContextFactory {
            override fun createContext(egl: EGL10?, display: EGLDisplay?, eglConfig: EGLConfig?): EGLContext {
                Log.i("egui_view", "createContext: egl: $egl, display: $display, eglConfig: $eglConfig")

                val ctx = egl!!.eglCreateContext(
                    display,
                    eglConfig,
                    EGL10.EGL_NO_CONTEXT,
                    intArrayOf(EGL_CONTEXT_CLIENT_VERSION, glVersion.toInt(), EGL10.EGL_NONE))
                nativeContext = NativeGLRenderer.createNativeContext0()
                if (nativeContext == 0L) {
                    throw RuntimeException("Failed to initialize native context")
                }
                return ctx
            }

            override fun destroyContext(egl: EGL10?, display: EGLDisplay?, context: EGLContext?) {
                Log.i("egui_view", "destroyContext: egl: $egl, display: $display, context: $context")
                NativeGLRenderer.destroyNativeContext0(nativeContext)
                nativeContext = 0L
                egl!!.eglDestroyContext(display, context)
            }
        })

        renderer = NativeGLRenderer()

        setRenderer(renderer)
    }
}

class NativeGLRenderer : GLSurfaceView.Renderer {
    override fun onDrawFrame(gl: GL10) {
        onDrawFrame0()
    }

    override fun onSurfaceCreated(gl: GL10?, conig: EGLConfig?) {
        Log.i("egui_view", "onSurfaceCreated")
        onSurfaceCreated0()
    }

    override fun onSurfaceChanged(gl: GL10, width: Int, height: Int) {
        Log.i("egui_view", "onSurfaceChanged: width: $width, height: $height")
        onSurfaceChanged0(width, height)
    }

    companion object {
        init {
            System.loadLibrary("native_gl_surface")
        }

        @JvmStatic
        external fun createNativeContext0(): Long
        @JvmStatic
        external fun destroyNativeContext0(handle: Long)

        @JvmStatic
        private external fun onDrawFrame0(handle: Long)
        @JvmStatic
        private external fun onSurfaceCreated0(handle: Long)
        @JvmStatic
        private external fun onSurfaceChanged0(handle: Long, width: Int, height: Int)
    }
}
