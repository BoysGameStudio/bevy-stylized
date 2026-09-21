package org.bevystylized.smoke;

import com.google.androidgamesdk.GameActivity;

public class MainActivity extends GameActivity {
    static { System.loadLibrary("stylized_android_smoke"); }
}
