package hk.jennyemily.work.lowu;

import android.content.Context;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.preference.PreferenceManager;
import android.util.Log;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;

import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.OutputStreamWriter;

import androidx.appcompat.app.AppCompatActivity;
import androidx.appcompat.widget.Toolbar;

public class LoadSSHActivity extends AppCompatActivity {
    private final static String TAG = "fanling10-load-SSH";

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_load_ssh);
        final Toolbar toolbar = findViewById(R.id.toolbar);
        setSupportActionBar(toolbar);
        final Button button = (Button) findViewById(R.id.buttonAddFiles);
        button.setOnClickListener(new View.OnClickListener() {
            public void onClick(View v) {
                SharedPreferences sp = PreferenceManager.getDefaultSharedPreferences(getBaseContext());
                final String ssh_path = sp.getString("ssh_path", "??");
                if (ssh_path != "??") {
                    final Context context = getApplicationContext();
                    uploadUsingEditText(R.id.editTextPrivateKey, ssh_path, context);
                    uploadUsingEditText(R.id.editTextPublicKey, ssh_path + ".pub", context);
                }
            }
        });
    }

    private void uploadUsingEditText(int id, String filePath, Context context) {
        final EditText editText = (EditText) findViewById(id);
        final String key = editText.getText().toString();
        writeToFile(filePath, key, context);
    }

    private void writeToFile(String filePath, String data, Context context) {
        try {
            Log.d(TAG, "Writing to " + filePath + "...");
            File file = new File(getFilesDir(), filePath);
            Log.d(TAG, "Writing to " + file.getAbsolutePath() +"...");
            FileOutputStream fileOutputStream = new FileOutputStream(file);
            OutputStreamWriter outputStreamWriter = new OutputStreamWriter(fileOutputStream);
            Log.d(TAG, "Writing '" + data.substring(0, 20) + "'...");
            outputStreamWriter.write(data);
            outputStreamWriter.close();
            Log.d(TAG, "Written to " + filePath + ".");
        } catch (
                IOException e) {
            Log.e("Exception", "File write failed: " + e.toString());
        }
    }
}
