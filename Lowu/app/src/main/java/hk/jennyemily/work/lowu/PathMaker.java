package hk.jennyemily.work.lowu;

import android.content.Context;

/**
 * returns paths to files used by the app
 */
public class PathMaker {
    private final static String TAG = "Lowu path maker";

    public static String pathName(Kind kind, Context context) {
        switch (kind) {
            case PUBLIC_KEY:
                return context.getFilesDir() + "/id_rsa.pub";
            case PRIVATE_KEY:
                return context.getFilesDir() + "/id_rsa";
            case ROOT:
                return context.getFilesDir() + "/";
        }
        return "??";
    }

    public enum Kind {PUBLIC_KEY, PRIVATE_KEY, ROOT}
}
