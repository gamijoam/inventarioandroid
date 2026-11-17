package com.inventario.apkinstaller

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

class ApkFileProvider : FileProvider()

@InvokeArg
class InstallArgs {
    var path: String = ""
}

@TauriPlugin
class ApkInstallerPlugin(private val activity: Activity) : Plugin(activity) {

    @Command
    fun install(invoke: Invoke) {
        try {
            val args = invoke.parseArgs(InstallArgs::class.java)
            val file = File(args.path)
            if (!file.exists()) {
                invoke.reject("No se encontró el archivo descargado")
                return
            }

            // Android 8+: el usuario debe permitir "instalar apps desconocidas" a esta app.
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O &&
                !activity.packageManager.canRequestPackageInstalls()
            ) {
                val settings = Intent(
                    Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES,
                    Uri.parse("package:" + activity.packageName)
                )
                activity.startActivity(settings)
                invoke.resolve(JSObject().put("status", "permiso"))
                return
            }

            val uri = FileProvider.getUriForFile(
                activity,
                activity.packageName + ".apkinstaller.fileprovider",
                file
            )
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/vnd.android.package-archive")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
            }
            activity.startActivity(intent)
            invoke.resolve(JSObject().put("status", "instalando"))
        } catch (e: Exception) {
            invoke.reject(e.message ?: "No se pudo abrir el instalador")
        }
    }
}
