use jni::objects::{JObject, JValue};
use jni::JavaVM;

pub fn save_to_downloads(filename: &str, contents: &str) -> Result<String, String> {
    let context = ndk_context::android_context();
    let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }
        .map_err(|error| format!("获取 Android JVM 失败: {error}"))?;
    let mut env = vm
        .attach_current_thread()
        .map_err(|error| format!("连接 Android JVM 失败: {error}"))?;

    let context_object = unsafe { JObject::from_raw(context.context().cast()) };
    let result = save_with_env(&mut env, &context_object, filename, contents);
    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_describe();
        let _ = env.exception_clear();
    }
    result
}

fn save_with_env(
    env: &mut jni::JNIEnv<'_>,
    context: &JObject<'_>,
    filename: &str,
    contents: &str,
) -> Result<String, String> {
    let uri_string = env
        .new_string("content://media/external/downloads")
        .map_err(java_error)?;
    let uri_class = env.find_class("android/net/Uri").map_err(java_error)?;
    let collection = env
        .call_static_method(
            uri_class,
            "parse",
            "(Ljava/lang/String;)Landroid/net/Uri;",
            &[JValue::Object(&uri_string)],
        )
        .map_err(java_error)?
        .l()
        .map_err(java_error)?;

    let values = env
        .new_object("android/content/ContentValues", "()V", &[])
        .map_err(java_error)?;
    for (key, value) in [
        ("_display_name", filename),
        ("mime_type", "application/x-go-sgf"),
        ("relative_path", "Download/LazyGo"),
    ] {
        let key = env.new_string(key).map_err(java_error)?;
        let value = env.new_string(value).map_err(java_error)?;
        env.call_method(
            &values,
            "put",
            "(Ljava/lang/String;Ljava/lang/String;)V",
            &[JValue::Object(&key), JValue::Object(&value)],
        )
        .map_err(java_error)?;
    }

    let resolver = env
        .call_method(
            context,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .map_err(java_error)?
        .l()
        .map_err(java_error)?;
    let inserted = env
        .call_method(
            &resolver,
            "insert",
            "(Landroid/net/Uri;Landroid/content/ContentValues;)Landroid/net/Uri;",
            &[JValue::Object(&collection), JValue::Object(&values)],
        )
        .map_err(java_error)?
        .l()
        .map_err(java_error)?;

    let stream_result = (|| -> Result<(), String> {
        let stream = env
            .call_method(
                &resolver,
                "openOutputStream",
                "(Landroid/net/Uri;)Ljava/io/OutputStream;",
                &[JValue::Object(&inserted)],
            )
            .map_err(java_error)?
            .l()
            .map_err(java_error)?;
        let bytes = env
            .byte_array_from_slice(contents.as_bytes())
            .map_err(java_error)?;
        let bytes = JObject::from(bytes);
        env.call_method(&stream, "write", "([B)V", &[JValue::Object(&bytes)])
            .map_err(java_error)?;
        env.call_method(&stream, "close", "()V", &[])
            .map_err(java_error)?;
        Ok(())
    })();

    if let Err(error) = stream_result {
        let _ = env.call_method(
            &resolver,
            "delete",
            "(Landroid/net/Uri;Ljava/lang/String;[Ljava/lang/String;)I",
            &[
                JValue::Object(&inserted),
                JValue::Object(&JObject::null()),
                JValue::Object(&JObject::null()),
            ],
        );
        return Err(error);
    }

    Ok(format!("Download/LazyGo/{filename}"))
}

fn java_error(error: jni::errors::Error) -> String {
    let detail = match error {
        jni::errors::Error::JavaException => "Android 写入失败".to_string(),
        other => other.to_string(),
    };
    format!("保存 SGF 失败: {detail}")
}
