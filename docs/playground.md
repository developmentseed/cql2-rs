<script src="https://ajax.googleapis.com/ajax/libs/jquery/3.7.1/jquery.min.js"></script>
<link href="https://cdnjs.cloudflare.com/ajax/libs/select2/4.0.13/css/select2.min.css" rel="stylesheet" />
<script src="https://cdnjs.cloudflare.com/ajax/libs/select2/4.0.13/js/select2.min.js"></script>

<style>
.select2-container {
    max-width: 100%; /* Makes it adapt to screen size */
    width: auto !important; /* Overrides any fixed width */
    min-width: 200px; /* Ensures it doesn’t get too small */
}

.select2-container--default .select2-selection--single {
    height: 34px !important; /* Keeps it aligned with the text input */
    font-size: 14px;
}

.select2-dropdown {
    min-width: 100% !important; /* Forces dropdown to match input */
    max-width: 600px; /* Prevents it from being too wide */
}

.select2-search__field {
    font-size: 14px !important;
    padding: 4px !important;
}

/* Media Queries to adjust for different screen sizes */
@media (max-width: 768px) {
    .select2-container {
        max-width: 90%; /* Takes most of the screen width on mobile */
    }
}

@media (max-width: 480px) {
    .select2-container {
        max-width: 100%; /* Full width on small screens */
    }
}
</style>

<script type="module">
  import init, { CQL2 } from '../pkg/cql2_wasm.js';

  await init();
  window.CQL2 = CQL2;
  $(document).ready(function(){
      console.log("Ready");
      console.log("window.cql2", window.CQL2);

      function check(){
          let valid = false;
          let txt = "Invalid";
          let jsn = "Invalid";
          try {
              let val = $("#cqlin").val();
              console.log("cqlin val", val);
              let e = new window.CQL2(val);
              valid = e.is_valid();
              txt = e.to_text();
              jsn = e.to_json_pretty();
          } catch(error) {
              console.log(error);
          }

          console.log(valid, txt, jsn);
          $("#cqlvalid").prop("checked", valid);
          $("#cql2text").val(txt).css({"background-color": valid ? "#90EE90" : "pink"});
          $("#cql2json").val(jsn).css({"background-color": valid ? "#90EE90" : "pink"});
      }

      $("#cqlin").on('input propertychange', check);

      $("#examples").change(function(){
          let selectedOption = $('#examples').find(":selected");
          let sel = selectedOption.val();
          let description = selectedOption.attr("title"); // Get the description

          if (sel.startsWith("{")) {
              let j = JSON.parse(sel);
              sel = JSON.stringify(j, null, 2);
          }

          $("#cqlin").val(sel);
          $("#examples").prop("selectedIndex", 0);
          $("#example-description").text("Current example description: " + description); // Set the description above the CQL input
          check();
      });

      // Initialize Select2
      $('#examples').select2({
          placeholder: "Search or select an example...",
          allowClear: true,
          width: '100%'
      });

      check();
  });
</script>

<h1>CQL2 Playground</h1>

<p id="example-description" style="font-weight: margin-bottom: 5px;"></p>

Examples: 

<select id="examples" class="searchable-dropdown" >
<option value=''>-</option>
<option value="{  &quot;op&quot;: &quot;a_overlaps&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;values&quot; },    [ { &quot;timestamp&quot;: &quot;2012-08-10T05:30:00Z&quot; }, { &quot;date&quot;: &quot;2010-02-10&quot; }, false ]  ]}" title="Checks for overlapping attribute values within a specified date range.">Overlapping Attribute Values Check</option>
<option value="{  &quot;op&quot;: &quot;in&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;eo:cloud_cover&quot; },    [ 0.1, 0.2 ]  ]}" title="Filters features based on a property being in a specified list.">Property List Filter</option>
<option value="{  &quot;op&quot;: &quot;s_crosses&quot;,  &quot;args&quot;: [    {      &quot;type&quot;: &quot;LineString&quot;,      &quot;coordinates&quot;: [        [ 43.72992, -79.2998 ], [ 43.73005, -79.2991 ], [ 43.73006, -79.2984 ],        [ 43.73140, -79.2956 ], [ 43.73259, -79.2950 ], [ 43.73266, -79.2945 ],        [ 43.73320, -79.2936 ], [ 43.73378, -79.2936 ], [ 43.73486, -79.2917 ]      ]    },    {      &quot;type&quot;: &quot;Polygon&quot;,      &quot;coordinates&quot;: [        [          [ 43.7286, -79.2986 ], [ 43.7311, -79.2996 ], [ 43.7323, -79.2972 ],          [ 43.7326, -79.2971 ], [ 43.7350, -79.2981 ], [ 43.7350, -79.2982 ],          [ 43.7352, -79.2982 ], [ 43.7357, -79.2956 ], [ 43.7337, -79.2948 ],          [ 43.7343, -79.2933 ], [ 43.7339, -79.2923 ], [ 43.7327, -79.2947 ],          [ 43.7320, -79.2942 ], [ 43.7322, -79.2937 ], [ 43.7306, -79.2930 ],          [ 43.7303, -79.2930 ], [ 43.7299, -79.2928 ], [ 43.7286, -79.2986 ]        ]      ]    }  ]}" title="Checks if a line string crosses a specified polygon.">Line String Crosses Polygon Check</option>
<option value="{ &quot;op&quot;: &quot;avg&quot;, &quot;args&quot;: [ { &quot;property&quot;: &quot;windSpeed&quot; } ] }" title="Computes the average of a specified property.">Average Property Calculation</option>
<option value="{  &quot;op&quot;: &quot;t_during&quot;,  &quot;args&quot;: [    {&quot;interval&quot;: [{ &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; }]},    {&quot;interval&quot;: [&quot;2005-01-10&quot;, &quot;2010-02-10&quot;]    }  ]}" title="Filters features based on a time interval condition.">Time Interval Filter</option>
<option value="{  &quot;op&quot;: &quot;isNull&quot;,  &quot;args&quot;: [ { &quot;property&quot;: &quot;value&quot; } ]}" title="Performs a isNull operation on properties.">isNull Operation</option>
<option value="{  &quot;op&quot;: &quot;and&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;=&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;swimming_pool&quot; },        true      ]    },    {      &quot;op&quot;: &quot;or&quot;,      &quot;args&quot;: [        {          &quot;op&quot;: &quot;>&quot;,          &quot;args&quot;: [            { &quot;property&quot;: &quot;floors&quot; },            5          ]        },        {          &quot;op&quot;: &quot;like&quot;,          &quot;args&quot;: [            { &quot;property&quot;: &quot;material&quot; },            &quot;brick%&quot;          ]        },        {          &quot;op&quot;: &quot;like&quot;,          &quot;args&quot;: [            { &quot;property&quot;: &quot;material&quot; },            &quot;%brick&quot;          ]        }      ]    }  ]}" title="Performs a and operation on properties.">AND Operation</option>
<option value="{  &quot;op&quot;: &quot;t_intersects&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] },    { &quot;interval&quot;: [ &quot;1991-10-07T08:21:06.393262Z&quot;, &quot;2010-02-10T05:29:20.073225Z&quot; ] }  ]}" title="Performs a t_intersects operation on properties.">Time Intersects Check</option>
<option value="{  &quot;op&quot;: &quot;>=&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;updated&quot; },    { &quot;date&quot;: &quot;1970-01-01&quot; }  ]}" title="Performs a >= operation on properties.">Greater Than or Equal Check</option>
<option value="{  &quot;op&quot;: &quot;not&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;like&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;name&quot; },        &quot;foo%&quot;      ]    }  ]}" title="Performs a not operation on properties.">NOT Operation</option>
<option value="{  &quot;op&quot;: &quot;not&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;in&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;category&quot; },        [ 1, 2, 3, 4 ]      ]    }  ]}" title="Performs a not operation on properties.">NOT IN Operation</option>
<option value="{  &quot;op&quot;: &quot;t_before&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;built&quot; },    { &quot;date&quot;: &quot;2015-01-01&quot; }  ]}" title="Performs a t_before operation on properties.">Time Before Check</option>
<option value="{  &quot;op&quot;: &quot;=&quot;,  &quot;args&quot;: [    0,    {      &quot;op&quot;: &quot;%&quot;,      &quot;args&quot;: [ { &quot;property&quot;: &quot;foo&quot; }, 2 ]    }  ]}" title="Performs a = operation on properties.">Equals Operation</option>
<option value="{  &quot;op&quot;: &quot;<=&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    {      &quot;op&quot;: &quot;^&quot;,      &quot;args&quot;: [ 2, { &quot;property&quot;: &quot;foo&quot; } ]    }  ]}" title="Performs a <= operation on properties.">Less Than or Equal Check</option>
<option value="{  &quot;op&quot;: &quot;t_after&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;built&quot; },    { &quot;date&quot;: &quot;2012-06-05&quot; }  ]}" title="Performs a t_after operation on properties.">Time After Check</option>
<option value="{  &quot;op&quot;: &quot;between&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    10, 20  ]}" title="Performs a between operation on properties.">Between Operation</option>
<option value="{  &quot;op&quot;: &quot;t_finishes&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] },    { &quot;interval&quot;: [ &quot;1991-10-07&quot;, &quot;2010-02-10T05:29:20.073225Z&quot; ] }  ]}" title="Performs a t_finishes operation on properties.">Time Finishes Check</option>
<option value="{  &quot;op&quot;: &quot;or&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;and&quot;,      &quot;args&quot;: [        {          &quot;op&quot;: &quot;>&quot;,          &quot;args&quot;: [            { &quot;property&quot;: &quot;floors&quot; },            5          ]        },        {          &quot;op&quot;: &quot;=&quot;,          &quot;args&quot;: [            { &quot;property&quot;: &quot;material&quot; },            &quot;brick&quot;          ]        }      ]    },    {      &quot;op&quot;: &quot;=&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;swimming_pool&quot; },        true      ]    }  ]}" title="Performs a or operation on properties.">OR Operation</option>
<option value="{  &quot;op&quot;: &quot;t_disjoint&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;..&quot;, &quot;2005-01-10T01:01:01.393216Z&quot; ] },    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] }  ]}" title="Performs a t_disjoint operation on properties.">Time Disjoint Check</option>
<option value="{  &quot;op&quot;: &quot;like&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;name&quot; },    &quot;Smith%&quot;  ]}" title="Performs a like operation on properties.">LIKE Operation</option>
<option value="{  &quot;op&quot;: &quot;t_during&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;1969-07-20T20:17:40Z&quot;, &quot;1969-07-21T17:54:00Z&quot; ] },    { &quot;interval&quot;: [ &quot;1969-07-16T13:32:00Z&quot;, &quot;1969-07-24T16:50:35Z&quot; ] }  ]}" title="Filters features based on a time interval condition.">Time Interval Condition Filter</option>
<option value="{  &quot;op&quot;: &quot;s_equals&quot;,  &quot;args&quot;: [    {      &quot;type&quot;: &quot;MultiPoint&quot;,      &quot;coordinates&quot;: [ [ 180.0, -0.5 ],                       [ 179.0, -47.121701 ],                       [ 180.0, -0.0 ],                       [ 33.470475, -0.99999 ],                       [ 179.0, -15.333062 ] ]    },    { &quot;property&quot;: &quot;geometry&quot; }  ]}" title="Performs a s_equals operation on properties.">Spatial Equals Check</option>
<option value="{  &quot;op&quot;: &quot;t_starts&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] },    { &quot;interval&quot;: [ &quot;1991-10-07T08:21:06.393262Z&quot;, &quot;..&quot; ] }  ]}" title="Performs a t_starts operation on properties.">Time Starts Check</option>
<option value="{  &quot;op&quot;: &quot;<&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;avg&quot;,      &quot;args&quot;: [ { &quot;property&quot;: &quot;windSpeed&quot; } ]    },    4  ]}" title="Performs a < operation on properties.">Less Than Check</option>
<option value="{  &quot;op&quot;: &quot;<>&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;id&quot; },    &quot;fa7e1920-9107-422d-a3db-c468cbc5d6df&quot;  ]}" title="Performs a <> operation on properties.">Not Equal Operation</option>
<option value="{  &quot;op&quot;: &quot;s_disjoint&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;MultiPolygon&quot;,      &quot;coordinates&quot;: [ [ [ [ 144.022387, 45.176126 ],                           [ -1.1, 0.0 ],                           [ 180.0, 47.808086 ],                           [ 144.022387, 45.176126 ] ] ] ]    }  ]}" title="Performs a s_disjoint operation on properties.">Spatial Disjoint Check</option>
<option value="{  &quot;op&quot;: &quot;s_overlaps&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    { &quot;bbox&quot;: [ -179.912109, 1.9, 180.0, 16.897016 ] }  ]}" title="Performs a s_overlaps operation on properties.">Spatial Overlaps Check</option>
<option value="{  &quot;op&quot;: &quot;s_intersects&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;Point&quot;,      &quot;coordinates&quot;: [ 36.319836, 32.288087 ]    }  ]}" title="Performs a s_intersects operation on properties.">Spatial Intersects Check</option>
<option value="{  &quot;op&quot;: &quot;s_contains&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;Point&quot;,      &quot;coordinates&quot;: [ -3.508362, -1.754181 ]    }  ]}" title="Performs a s_contains operation on properties.">Spatial Contains Check</option>
<option value="{  &quot;op&quot;: &quot;t_contains&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;2000-01-01T00:00:00Z&quot;, &quot;2005-01-10T01:01:01.393216Z&quot; ] },    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] }      ]}" title="Performs a t_contains operation on properties.">Time Contains Check</option>
<option value="{  &quot;op&quot;: &quot;s_touches&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;MultiLineString&quot;,      &quot;coordinates&quot;: [ [ [ -1.9, -0.99999 ],                         [ 75.292574, 1.5 ],                         [ -0.5, -4.016458 ],                         [ -31.708594, -74.743801 ],                         [ 179.0, -90.0 ] ],                       [ [ -1.9, -1.1 ],                         [ 1.5, 8.547371 ] ] ]    }  ]}" title="Performs a s_touches operation on properties.">Spatial Touches Check</option>
<option value="{  &quot;op&quot;: &quot;t_overlaps&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] },    { &quot;interval&quot;: [ &quot;1991-10-07T08:21:06.393262Z&quot;, &quot;1992-10-09T08:08:08.393473Z&quot; ] }  ]}" title="Performs a t_overlaps operation on properties.">Time Overlaps Check</option>
<option value="{  &quot;op&quot;: &quot;>&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;-&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;balance&quot; },        150.0      ]    },    0  ]}" title="Performs a > operation on properties.">Greater Than Check</option>
<option value="{  &quot;op&quot;: &quot;<&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    10  ]}" title="Performs a < operation on properties.">Less Than Check</option>
<option value="{  &quot;op&quot;: &quot;t_startedBy&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;1991-10-07T08:21:06.393262Z&quot;, &quot;2010-02-10T05:29:20.073225Z&quot; ] },    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] }  ]}" title="Performs a t_startedBy operation on properties.">Time StartedBy Check</option>
<option value="{  &quot;op&quot;: &quot;s_within&quot;,  &quot;args&quot;: [    {      &quot;type&quot;: &quot;Polygon&quot;,      &quot;coordinates&quot;: [ [ [ -49.88024, 0.5, -75993.341684 ],                         [ -1.5, -0.99999, -100000.0 ],                         [ 0.0, 0.5, -0.333333 ],                         [ -49.88024, 0.5, -75993.341684 ] ],                       [ [ -65.887123, 2.00001, -100000.0 ],                         [ 0.333333, -53.017711, -79471.332949 ],                         [ 180.0, 0.0, 1852.616704 ],                         [ -65.887123, 2.00001, -100000.0 ] ] ]    },    { &quot;property&quot;: &quot;geometry&quot; }  ]}" title="Performs a s_within operation on properties.">Spatial Within Check</option>
<option value="{  &quot;op&quot;: &quot;<&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    {      &quot;op&quot;: &quot;-&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;foo&quot; },        10      ]    }  ]}" title="Performs a < operation on properties.">Less Than Check</option>
<option value="{  &quot;op&quot;: &quot;s_intersects&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;Polygon&quot;,      &quot;coordinates&quot;: [ [ [ -10, -10 ], [ 10, -10 ], [ 10, 10 ], [ -10, -10 ] ] ]    }  ]}" title="Performs a s_intersects operation on properties.">Spatial Intersects Check</option>
<option value="{  &quot;op&quot;: &quot;>&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    10  ]}" title="Performs a > operation on properties.">Greater Than Check</option>
<option value="{  &quot;op&quot;: &quot;like&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;owner&quot; },    &quot;Mike%&quot;  ]}" title="Performs a like operation on properties.">LIKE Operation</option>
<option value="{  &quot;op&quot;: &quot;s_intersects&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    { &quot;bbox&quot;: [ -128.098193, -1.1, -99999.0, 180.0, 90.0, 100000.0 ] }  ]}" title="Performs a s_intersects operation on properties.">Spatial Intersects Check</option>
<option value="{  &quot;op&quot;: &quot;t_after&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;updated_at&quot; },    { &quot;date&quot;: &quot;2010-02-10&quot; }  ]}" title="Performs a t_after operation on properties.">Time After Check</option>
<option value="{  &quot;op&quot;: &quot;in&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;casei&quot;,      &quot;args&quot;: [ { &quot;property&quot;: &quot;road_class&quot; } ]    },    [      { &quot;op&quot;: &quot;casei&quot;, &quot;args&quot;: [ &quot;Οδος&quot; ] },      { &quot;op&quot;: &quot;casei&quot;, &quot;args&quot;: [ &quot;Straße&quot; ] }    ]  ]}" title="Filters features based on a property being in a specified list.">Property List Filter</option>
<option value="{  &quot;op&quot;: &quot;like&quot;,  &quot;args&quot;: [    { &quot;op&quot;: &quot;casei&quot;, &quot;args&quot;: [ { &quot;property&quot;: &quot;geophys:SURVEY_NAME&quot; } ] },    { &quot;op&quot;: &quot;casei&quot;, &quot;args&quot;: [ &quot;%calcutta%&quot; ] }  ]}" title="Performs a like operation on properties.">LIKE Operation</option>
<option value="{  &quot;op&quot;: &quot;t_intersects&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;event_time&quot; },    { &quot;interval&quot;: [ &quot;1969-07-16T05:32:00Z&quot;, &quot;1969-07-24T16:50:35Z&quot; ] }  ]}" title="Performs a t_intersects operation on properties.">Time Intersects Check</option>
<option value="{  &quot;op&quot;: &quot;<>&quot;,  &quot;args&quot;: [    false,    {      &quot;op&quot;: &quot;Bar&quot;,      &quot;args&quot;: [ { &quot;property&quot;: &quot;geometry&quot; }, 100, &quot;a&quot;, &quot;b&quot;, false ]    }  ]}" title="Performs a <> operation on properties.">Not Equal Operation</option>
<option value="{  &quot;op&quot;: &quot;like&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;name&quot; },    { &quot;op&quot;: &quot;casei&quot;, &quot;args&quot;: [ &quot;FOO%&quot; ] }  ]}" title="Performs a like operation on properties.">LIKE Operation</option>
<option value="{  &quot;op&quot;: &quot;t_during&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;touchdown&quot; }, { &quot;property&quot;: &quot;liftOff&quot; } ] },    { &quot;interval&quot;: [ &quot;1969-07-16T13:32:00Z&quot;, &quot;1969-07-24T16:50:35Z&quot; ] }  ]}" title="Filters features based on a time interval condition.">Time Interval Condition Filter</option>
<option value="{  &quot;op&quot;: &quot;s_contains&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;Point&quot;,      &quot;coordinates&quot;: [ -3.508362, -1.754181 ]    }  ]}" title="Performs a s_contains operation on properties.">Spatial Contains Check</option>
<option value="{  &quot;op&quot;: &quot;or&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;isNull&quot;,      &quot;args&quot;: [ { &quot;property&quot;: &quot;value&quot; } ]    },    {      &quot;op&quot;: &quot;between&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;value&quot; },        10, 20      ]    }  ]}" title="Performs a or operation on properties.">OR Operation</option>
<option value="{  &quot;op&quot;: &quot;not&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;like&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;owner&quot; },        &quot;%Mike%&quot;      ]    }  ]}" title="Performs a not operation on properties.">NOT Operation</option>
<option value="{  &quot;op&quot;: &quot;t_overlappedBy&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;1991-10-07T08:21:06.393262Z&quot;, &quot;2010-02-10T05:29:20.073225Z&quot; ] },    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] }  ]}" title="Performs a t_overlappedBy operation on properties.">Time OverlappedBy Check</option>
<option value="{  &quot;op&quot;: &quot;<=&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    10  ]}" title="Performs a <= operation on properties.">Less Than or Equal Check</option>
<option value="{  &quot;op&quot;: &quot;>&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;value&quot; },    {      &quot;op&quot;: &quot;+&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;foo&quot; },        10      ]    }  ]}" title="Performs a > operation on properties.">Greater Than Check</option>
<option value="{  &quot;op&quot;: &quot;>&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;floors&quot; },    5  ]}" title="Performs a > operation on properties.">Greater Than Check</option>
<option value="{  &quot;op&quot;: &quot;and&quot;,  &quot;args&quot;: [    {      &quot;op&quot;: &quot;between&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;eo:cloud_cover&quot; },        0.1, 0.2      ]    },    {      &quot;op&quot;: &quot;=&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;landsat:wrs_row&quot; },        28      ]    },    {      &quot;op&quot;: &quot;=&quot;,      &quot;args&quot;: [        { &quot;property&quot;: &quot;landsat:wrs_path&quot; },        203      ]    }  ]}" title="Performs a and operation on properties.">AND Operation</option>
<option value="{  &quot;op&quot;: &quot;s_touches&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;geometry&quot; },    {      &quot;type&quot;: &quot;MultiLineString&quot;,      &quot;coordinates&quot;: [ [ [ -1.9, -0.99999 ],                         [ 75.292574, 1.5 ],                         [ -0.5, -4.016458 ],                         [ -31.708594, -74.743801 ],                         [ 179.0, -90.0 ] ],                       [ [ -1.9, -1.1 ],                         [ 1.5, 8.547371 ] ] ]    }  ]}" title="Performs a s_touches operation on properties.">Spatial Touches Check</option>
<option value="{  &quot;op&quot;: &quot;=&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;swimming_pool&quot; },    true  ]}" title="Performs a = operation on properties.">Equals Operation</option>
<option value="{  &quot;op&quot;: &quot;t_contains&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;2000-01-01T00:00:00Z&quot;, &quot;2005-01-10T01:01:01.393216Z&quot; ] },    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] }      ]}" title="Performs a t_contains operation on properties.">Time Contains Check</option>
<option value="{  &quot;op&quot;: &quot;t_equals&quot;,  &quot;args&quot;: [    { &quot;property&quot;: &quot;updated_at&quot; },    { &quot;date&quot;: &quot;1851-04-29&quot; }  ]}" title="Performs a t_equals operation on properties.">Time Equals Check</option>
<option value="{  &quot;op&quot;: &quot;t_metBy&quot;,  &quot;args&quot;: [    { &quot;interval&quot;: [ &quot;2010-02-10T05:29:20.073225Z&quot;, &quot;2010-10-07&quot; ] },    { &quot;interval&quot;: [ { &quot;property&quot;: &quot;starts_at&quot; }, { &quot;property&quot;: &quot;ends_at&quot; } ] }  ]}" title="Performs a t_metBy operation on properties.">Time MetBy Check</option>

  </select>
  <textarea id="cqlin" rows="20" cols="100">foo > 1</textarea>
  <br/>
  Valid:<input type="checkbox" id="cqlvalid" onclick="return false"></input>
  <br/>
  Parsed CQL2 Text
  <br/>
  <textarea id="cql2text" rows="10" cols="100" readonly>Parsed CQL2 Text</textarea>
  <br/>
  Parsed CQL2 JSON
  <br/>
  <textarea id="cql2json" rows="20" cols="100" readonly>Parsed CQL2 JSON</textarea>
