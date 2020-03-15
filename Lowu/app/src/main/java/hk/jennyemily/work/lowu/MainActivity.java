/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */
package hk.jennyemily.work.lowu;

import android.app.Activity;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.preference.PreferenceManager;
import android.util.Log;
import android.view.Menu;
import android.view.MenuItem;
import android.webkit.JavascriptInterface;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import org.json.JSONException;
import org.json.JSONObject;

import androidx.appcompat.app.AppCompatActivity;
import androidx.appcompat.widget.Toolbar;
import taiposwig.CCycleEvent;
import taiposwig.CResponseItem;

public class MainActivity extends AppCompatActivity {
    static {
        System.loadLibrary("taiposwig");
    }

    private taiposwig.SWIGTYPE_p_LowuData td;
    private final static String TAG = "fanling10";
    private WebView mWebView;
    private static final int RESULT_SETTINGS = 1;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        Log.d(TAG, "on create...");
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        Toolbar toolbar = findViewById(R.id.toolbar);
        toolbar.setTitle("Fanling10");
        setSupportActionBar(toolbar);

        setInitialPreferencesIfRequired();

        Log.d(TAG, "making data...");
        td = taiposwig.taiposwig.make_data(jsonOptions().toString());
        Log.d(TAG, "data made.");

               WebView mWebView = findViewById(R.id.webview);
        WebSettings webSettings = mWebView.getSettings();
        webSettings.setJavaScriptEnabled(true);
        mWebView.addJavascriptInterface(new WebAppInterface(this), "taipo");
        //!!! add some code here to set platform-specific javascript (look at the PC version)
        mWebView.setWebViewClient(new WebViewClient());

                Log.d(TAG, "web view set");
        String ih = taiposwig.taiposwig.initial_html(td);
        Log.d(TAG, "have initial html");
//     Log.d(TAG, "initial html: "+ih);
        mWebView.loadData(ih, null, null);

        taiposwig.taiposwig.handle_event(td, CCycleEvent.Start);

        Log.d(TAG, "on create done");
    }

    void setInitialPreferencesIfRequired() {
        Log.d(TAG, "setting initial preferences...");
        SharedPreferences sp = PreferenceManager.getDefaultSharedPreferences(getBaseContext());
        Log.d(TAG, "in main using: " + PreferenceManager.getDefaultSharedPreferencesName(getBaseContext()));
        if (sp.getString("unique_prefix", "") == "") {
        SharedPreferences.Editor ed = sp.edit();
        ed.putString("database_path", getApplicationContext().getDataDir() + "/search.db");
                ed.putString("git_path", getApplicationContext().getDataDir() + "/test2.git");
               ed.putString("git_branch", "master");
            ed.putBoolean("git_has_url", true);
        ed.putString("git_url", "git@test.jennyemily.hk:martin/data1.git");
        ed.putString("git_name", "martin");
        ed.putString("git_email", "m.e@acm.org");
        ed.putString("unique_prefix", "x");
        ed.putString("ssh_path", getApplicationContext().getDataDir() + "/id_rsa");
        ed.putBoolean("slurp_ssh", true);
        ed.apply();
        Log.d(TAG, "set initial preferences.");
        } else {
            Log.d(TAG, "preferences already set");
        }
    }

    JSONObject jsonOptions() {
        Log.d(TAG, "setting options...");
        JSONObject json = new JSONObject();
        SharedPreferences sp = PreferenceManager.getDefaultSharedPreferences(getBaseContext());
        Log.d(TAG, "in main using: " + PreferenceManager.getDefaultSharedPreferencesName(getBaseContext()));
        try {
            json.put("database_path", sp.getString("database_path", "??"));
            json.put("git_path", sp.getString("git_path", "??"));
            json.put("branch", sp.getString("git_branch", "??"));
            json.put("have_url", sp.getBoolean("git_have_url", false));
            json.put("url", sp.getString("git_url", "??"));
            json.put("name", sp.getString("git_name", "??"));
            json.put("email", sp.getString("git_email", "??"));
            json.put("unique_prefix", sp.getString("unique_prefix", "??"));
            json.put("ssh_path", sp.getString("ssh_path", "??"));
            json.put("slurp_ssh", sp.getBoolean("slurp_ssh", true));
            Log.d(TAG, "options set, prefix is " + sp.getString("unique_prefix", "??"));
        } catch (JSONException e) {
            e.printStackTrace();
        }
        return json;
    }

    @Override
    protected void onPause() {
        Log.d(TAG, "pausing...");
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Pause);
        super.onPause();
    }

    @Override
    protected void onResume() {
        Log.d(TAG, "resuming...");
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Resume);
        super.onResume();
    }

    @Override
    protected void onStart() {
        Log.d(TAG, "starting...");
        super.onStart();
    }

    @Override
    protected void onRestart() {
        Log.d(TAG, "restarting...");
        super.onRestart();
    }

    @Override
    protected void onStop() {
        Log.d(TAG, "stopping...");
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Stop);
        super.onStop();
    }

    @Override
    public boolean onCreateOptionsMenu(Menu menu) {
        // Inflate the menu; this adds items to the action bar if it is present.
        getMenuInflater().inflate(R.menu.menu_main, menu);
        return true;
    }

    @Override
    public boolean onOptionsItemSelected(MenuItem item) {
        // Handle action bar item clicks here. The action bar will
        // automatically handle clicks on the Home/Up button, so long
        // as you specify a parent activity in AndroidManifest.xml.
        int id = item.getItemId();
        switch (id) {
            case R.id.menuSettings:
                Log.d(TAG, "settings clicked");
                Intent intent = new Intent(MainActivity.this, PreferencesActivity.class);
                startActivityForResult(intent, RESULT_SETTINGS);
                break;
            default:
                Log.d(TAG, "some menu option selected");
        }
        return super.onOptionsItemSelected(item);
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);

        switch (requestCode) {
            case RESULT_SETTINGS:
                switch (resultCode) {
                    case PreferencesActivity.NOT_CHANGED_RESULT:
                        Log.d(TAG, "settings done, not changed result");
                        break;
                    case PreferencesActivity.CHANGED_RESULT:
                        Log.d(TAG, "settings done, changed result");
                        Intent i = new Intent(MainActivity.this, MainActivity.class);
                        i.setFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP);
                        i.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
                        MainActivity.this.startActivity(i);
                        break;
                    default:
                        Log.d(TAG, "settings done, unknown result " + resultCode);
                        break;
                }
                break;
            default:
                throw new IllegalStateException("Unexpected value: " + requestCode);
        }
    }

    @Override
    protected void onDestroy() {
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Destroy);
        Log.d(TAG, "on destroy, releasing rust data (" + (isFinishing() ? "finishing" : "not finishing") + ")...");
        taiposwig.taiposwig.delete_data(td);
        super.onDestroy();
        Log.d(TAG, "done.");
    }

    class WebAppInterface {
        WebAppInterface(Activity a) {
        }

        @JavascriptInterface
        public void execute(String body) {
            taiposwig.taiposwig.execute(td, body);
            if (taiposwig.taiposwig.is_shutdown_required(td)) {
                Log.d(TAG, "shutdown required!");
                finish();
            }
        }

        @JavascriptInterface
        public boolean response_ok() {
            return taiposwig.taiposwig.response_ok(td);
        }

        @JavascriptInterface
        public String response_error() {
            return taiposwig.taiposwig.response_error(td);
        }

        @JavascriptInterface
        public int response_num_items() {
            return taiposwig.taiposwig.response_num_items(td);
        }

        @JavascriptInterface
        public CResponseItem response_item(int n) {
            return taiposwig.taiposwig.response_item(td, n);
        }

        @JavascriptInterface
        public String response_key(CResponseItem cri) {
            return cri.getKey();
        }

        @JavascriptInterface
        public String response_value(CResponseItem cri) {
            return cri.getValue();
        }

    }

}
