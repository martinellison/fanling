/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

package hk.jennyemily.work.lowu;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
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
    String TAG = "fanling10";
    private WebView mWebView;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        Log.d(TAG, "on create...");
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);
        Toolbar toolbar = findViewById(R.id.toolbar);
        toolbar.setTitle("Fanling10");
        setSupportActionBar(toolbar);

        Log.d(TAG, "making data...");
        JSONObject json = new JSONObject();
        try {
            json.put("database_path", getApplicationContext().getDataDir() + "/search.db");
            //json.put("database_path", getApplicationContext().getDataDir() + "/test2.db");
            json.put("git_path", getApplicationContext().getDataDir() + "/test2.git");
            //json.put("git_path", getApplicationContext().getDataDir() + "/store.git");
            json.put("branch", "master");
            json.put("have_url", true);
            json.put("url", "git@test.jennyemily.hk:martin/data1.git");
            //json.put("url", "git@test.jennyemily.hk:fanling/testrep2.git");
            json.put("name", "martin");
            json.put("email", "m.e@acm.org");
            json.put("unique_prefix", "x");
            json.put("ssh_path", getApplicationContext().getDataDir() + "/id_rsa");
            json.put("slurp_ssh", true);
        } catch (JSONException e) {
            e.printStackTrace();
        }

        td = taiposwig.taiposwig.make_data(json.toString());
        Log.d(TAG, "data made.");

        // mWebView = new WebView(this);
        WebView mWebView = findViewById(R.id.webview);
        WebSettings webSettings = mWebView.getSettings();
        webSettings.setJavaScriptEnabled(true);
        mWebView.addJavascriptInterface(new WebAppInterface(this), "taipo");
        //!!! add some code here to set platform-specific javascript (look at the PC version)
        mWebView.setWebViewClient(new WebViewClient());

        //  setContentView(mWebView);
        Log.d(TAG, "web view set");
        String ih = taiposwig.taiposwig.initial_html(td);
        Log.d(TAG, "have initial html");
//     Log.d(TAG, "initial html: "+ih);
        mWebView.loadData(ih, null, null);

        taiposwig.taiposwig.handle_event(td, CCycleEvent.Start);

        Log.d(TAG, "on create done");
    }


    @Override
    protected void onPause() {
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Pause);
        super.onPause();
    }

    @Override
    protected void onResume() {
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Resume);
        super.onResume();
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
                startActivity(intent);
                break;
            default:
                Log.d(TAG, "some menu option selected");
        }
        return super.onOptionsItemSelected(item);
    }

    @Override
    protected void onDestroy() {
        taiposwig.taiposwig.handle_event(td, CCycleEvent.Stop);
        Log.d(TAG, "on destroy, releasing rust data...");
        taiposwig.taiposwig.delete_data(td);
        super.onDestroy();
        Log.d(TAG, "done.");
    }

    class WebAppInterface {
        WebAppInterface(Activity a) {
            //  mActivity = a;
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
