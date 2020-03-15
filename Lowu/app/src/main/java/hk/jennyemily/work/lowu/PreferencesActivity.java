/* This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/. */

package hk.jennyemily.work.lowu;

import android.app.Activity;
import android.content.SharedPreferences;
import android.os.Bundle;
import android.preference.CheckBoxPreference;
import android.preference.EditTextPreference;
import android.preference.Preference;
import android.preference.PreferenceActivity;
import android.preference.PreferenceManager;
import android.util.Log;

import static android.util.Log.d;

public class PreferencesActivity extends PreferenceActivity {
    private final static String TAG = "fanling10:PreferencesActivity";
    private static final String[] keys = {
            "git_path",
            "git_branch", "git_has_url", "git_url", "git_name", "git_email", "database_path",
            "unique_prefix", "ssh_path", "slurp_ssh"
    };
    public final static int NOT_CHANGED_RESULT = RESULT_FIRST_USER;
    public final static int CHANGED_RESULT = RESULT_FIRST_USER + 1;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
       /* setContentView(R.layout.activity_preferences);
        Toolbar toolbar = findViewById(R.id.toolbar);
        setSupportActionBar(toolbar);

        FloatingActionButton fab = findViewById(R.id.fab);
        fab.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                Snackbar.make(view, "Replace with your own action", Snackbar.LENGTH_LONG)
                        .setAction("Action", null).show();
            }
        });
        getSupportActionBar().setDisplayHomeAsUpEnabled(true);*/
        addPreferencesFromResource(R.xml.prefs);
        SharedPreferences sp = PreferenceManager.getDefaultSharedPreferences(getBaseContext());
        Log.d(TAG, "as edited: " + getPreferenceManager().getSharedPreferencesName());
        Log.d(TAG, "default: " + PreferenceManager.getDefaultSharedPreferencesName(getBaseContext()));
        for (String key : keys) {
            Preference pref = findPreference(key);
            //   Log.d(TAG, key + " is a " + pref.toString());
            pref.setOnPreferenceChangeListener(sChangeListener);
            if (pref instanceof EditTextPreference) {
                pref.setSummary(sp.getString(pref.getKey(), "??"));
                Log.d(TAG, pref.getKey() + ": " + sp.getString(pref.getKey(), "??"));
            } else if (pref instanceof CheckBoxPreference) {
                pref.setSummary(sp.getBoolean(pref.getKey(), false) ? "true" : "false");
            }
        }
        Log.d(TAG, "unique prefix: " + sp.getString("unique_prefix", "??"));
        setResult(NOT_CHANGED_RESULT);
    }

    private static Preference.OnPreferenceChangeListener sChangeListener = new Preference.OnPreferenceChangeListener() {
        @Override
        public boolean onPreferenceChange(Preference preference, Object newValue) {
            String stringValue = newValue.toString();
            d(TAG, "preference " + preference.getKey() + " changed to " + stringValue);
            preference.setSummary(stringValue);
            Activity activity = (Activity) preference.getContext();
            activity.setResult(CHANGED_RESULT);
            return true;
        }
    };
}
