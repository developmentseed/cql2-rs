<script src="https://ajax.googleapis.com/ajax/libs/jquery/3.7.1/jquery.min.js"></script>
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
                let val =$("#cqlin").val();
                console.log("cqlin val", val);
                let e = new window.CQL2(val);
                valid = e.is_valid();
                txt = e.to_text();
                jsn = e.to_json_pretty();
            } catch(error) {
                console.log(error);

            }
            console.log(valid, txt, jsn);
            if (valid) {
                $("#cqlvalid").prop("checked", true);
                $("#cql2text").css({"background-color": "#90EE90"});
                $("#cql2json").css({"background-color": "#90EE90"});
            } else {
                $("#cqlvalid").prop("checked", false);
                $("#cql2text").css({"background-color": "pink"});
                $("#cql2json").css({"background-color": "pink"});
            };

            $("#cql2text").val(txt);
            $("#cql2json").val(jsn);
        };
        $("#cqlin").bind('input propertychange', check);
        $("#examples").change(function(){
          let sel = $('#examples').find(":selected").val();
          if (sel.startsWith("{")){
            let j = JSON.parse(sel);
            sel = JSON.stringify(j, null, 2);
          }
          $("#cqlin").val(sel);
          $("#examples").prop("selectedIndex", 0);
          check();
        });
        check();
    });

  </script>
  <h1>CQL2 Playground</h1>
  Examples: <select id="examples">
<option value=''>-</option>


<option value="avg(windSpeed)">clause6_01.txt</option>
<option value="city='Toronto'">clause6_02a.txt</option>
<option value="avg(windSpeed) < 4">clause6_02b.txt</option>
<option value="balance-150.0 > 0">clause6_02c.txt</option>
<option value="updated >= date('1970-01-01')">clause6_02d.txt</option>
<option value="geometry IS NOT NULL">clause6_03.txt</option>
<option value="name LIKE 'Smith%'">clause7_01.txt</option>
<option value="depth BETWEEN 100.0 and 150.0">clause7_02.txt</option>
<option value="cityName IN ('Toronto','Frankfurt','Tokyo','New York')">clause7_03a.txt</option>
<option value="category NOT IN (1,2,3,4)">clause7_03b.txt</option>
<option value="CASEI(road_class) IN (CASEI('Οδος'),CASEI('Straße'))">clause7_04.txt</option>
<option value="ACCENTI(etat_vol) = ACCENTI('débárquér')">clause7_05.txt</option>
<option value="S_INTERSECTS(geometry,POINT(36.319836 32.288087))">clause7_07.txt</option>
<option value="S_CROSSES(road,POLYGON((43.7286 -79.2986, 43.7311 -79.2996, 43.7323 -79.2972,
                        43.7326 -79.2971, 43.7350 -79.2981, 43.7350 -79.2982,
                        43.7352 -79.2982, 43.7357 -79.2956, 43.7337 -79.2948,
                        43.7343 -79.2933, 43.7339 -79.2923, 43.7327 -79.2947,
                        43.7320 -79.2942, 43.7322 -79.2937, 43.7306 -79.2930,
                        43.7303 -79.2930, 43.7299 -79.2928, 43.7286 -79.2986)))">clause7_10.txt</option>
<option value="T_INTERSECTS(event_time, INTERVAL('1969-07-16T05:32:00Z', '1969-07-24T16:50:35Z'))">clause7_12.txt</option>
<option value="T_DURING(INTERVAL(touchdown, liftOff), INTERVAL('1969-07-16T13:32:00Z', '1969-07-24T16:50:35Z'))">clause7_13.txt</option>
<option value="A_CONTAINS(layer:ids, ('layers-ca','layers-us'))">clause7_15.txt</option>
<option value="S_CROSSES(LINESTRING(43.72992 -79.2998, 43.73005 -79.2991, 43.73006 -79.2984,
                     43.73140 -79.2956, 43.73259 -79.2950, 43.73266 -79.2945,
                     43.73320 -79.2936, 43.73378 -79.2936, 43.73486 -79.2917),
        POLYGON((43.7286 -79.2986, 43.7311 -79.2996, 43.7323 -79.2972, 43.7326 -79.2971,
                 43.7350 -79.2981, 43.7350 -79.2982, 43.7352 -79.2982, 43.7357 -79.2956,
                 43.7337 -79.2948, 43.7343 -79.2933, 43.7339 -79.2923, 43.7327 -79.2947,
                 43.7320 -79.2942, 43.7322 -79.2937, 43.7306 -79.2930, 43.7303 -79.2930,
                 43.7299 -79.2928, 43.7286 -79.2986)))">clause7_16.txt</option>
<option value="T_DURING(INTERVAL('1969-07-20T20:17:40Z', '1969-07-21T17:54:00Z'), INTERVAL('1969-07-16T13:32:00Z', '1969-07-24T16:50:35Z'))">clause7_17.txt</option>
<option value="S_WITHIN(road,Buffer(geometry,10,'m'))">clause7_18.txt</option>
<option value="vehicle_height > (bridge_clearance-1)">clause7_19.txt</option>
<option value="landsat:scene_id = 'LC82030282019133LGN00'">example01.txt</option>
<option value="eo:instrument LIKE 'OLI%'">example02.txt</option>
<option value="landsat:wrs_path IN ('153','154','15X')">example03.txt</option>
<option value="eo:cloud_cover < 0.1 AND landsat:wrs_row=28 AND landsat:wrs_path=203">example04.txt</option>
<option value="eo:cloud_cover=0.1 OR eo:cloud_cover=0.2">example05a.txt</option>
<option value="eo:cloud_cover IN (0.1,0.2)">example05b.txt</option>
<option value="    eo:cloud_cover BETWEEN 0.1 AND 0.2
AND landsat:wrs_row=28
AND landsat:wrs_path=203">example06a.txt</option>
<option value="    eo:cloud_cover >= 0.1
AND eo:cloud_cover <= 0.2
AND landsat:wrs_row=28
AND landsat:wrs_path=203">example06b.txt</option>
<option value="eo:instrument LIKE 'OLI%'
                AND S_INTERSECTS(footprint,POLYGON((43.5845 -79.5442,
                                                    43.6079 -79.4893,
                                                    43.5677 -79.4632,
                                                    43.6129 -79.3925,
                                                    43.6223 -79.3238,
                                                    43.6576 -79.3163,
                                                    43.7945 -79.1178,
                                                    43.8144 -79.1542,
                                                    43.8555 -79.1714,
                                                    43.7509 -79.6390,
                                                    43.5845 -79.5442)))">example07.txt</option>
<option value="    beamMode='ScanSAR Narrow'
AND swathDirection='ascending'
AND polarization='HH+VV+HV+VH'
AND s_intersects(footprint,POLYGON((-77.117938 38.936860,
                                    -77.040604 39.995648,
                                    -76.910536 38.892912,
                                    -77.039359 38.791753,
                                    -77.047906 38.841462,
                                    -77.034183 38.840655,
                                    -77.033142 38.857490,
                                    -77.117938 38.936860)))">example08.txt</option>
<option value="floors>5">example09.txt</option>
<option value="taxes <= 500">example10.txt</option>
<option value="owner LIKE '%Jones%'">example11.txt</option>
<option value="owner LIKE 'Mike%'">example12.txt</option>
<option value="owner NOT LIKE '%Mike%'">example13.txt</option>
<option value="swimming_pool = true">example14.txt</option>
<option value="floors>5 AND swimming_pool=true">example15.txt</option>
<option value="swimming_pool=true AND (floors>5
                    OR  material LIKE 'brick%'
                    OR  material LIKE '%brick')">example16.txt</option>
<option value="(floors>5 AND material='brick') OR swimming_pool=true">example17.txt</option>
<option value="NOT (floors<5) OR swimming_pool=true">example18.txt</option>
<option value="(owner LIKE 'mike%' OR owner LIKE 'Mike%') AND floors<4">example19.txt</option>
<option value="T_BEFORE(built, DATE('2015-01-01'))">example20.txt</option>
<option value="T_AFTER(built,DATE('2012-06-05'))">example21.txt</option>
<option value="T_DURING(INTERVAL(starts_at, ends_at), INTERVAL('2017-06-10T07:30:00Z', '2017-06-11T10:30:00Z'))">example22.txt</option>
<option value="S_WITHIN(location,BBOX(-118,33.8,-117.9,34))">example23.txt</option>
<option value="S_INTERSECTS(geometry,POLYGON((-10.0 -10.0,10.0 -10.0,10.0 10.0,-10.0 -10.0)))">example24.txt</option>
<option value="floors>5 AND S_WITHIN(geometry,BBOX(-118,33.8,-117.9,34))">example25.txt</option>
<option value="CASEI(road_class) IN (CASEI('Οδος'),CASEI('Straße'))">example26.txt</option>
<option value="ACCENTI(etat_vol) = ACCENTI('débárquér')">example27.txt</option>
<option value="CASEI(geophys:SURVEY_NAME) LIKE CASEI('%calcutta%')">example28.txt</option>
<option value="&quot;id&quot; = 'fa7e1920-9107-422d-a3db-c468cbc5d6df'">example29.txt</option>
<option value="&quot;id&quot; <> 'fa7e1920-9107-422d-a3db-c468cbc5d6df'">example30.txt</option>
<option value="&quot;value&quot; < 10">example31.txt</option>
<option value="&quot;value&quot; > 10">example32.txt</option>
<option value="&quot;value&quot; <= 10">example33.txt</option>
<option value="&quot;value&quot; >= 10">example34.txt</option>
<option value="&quot;name&quot; LIKE 'foo%'">example35.txt</option>
<option value="&quot;name&quot; NOT LIKE 'foo%'">example36-alt01.txt</option>
<option value="NOT &quot;name&quot; LIKE 'foo%'">example36.txt</option>
<option value="&quot;value&quot; BETWEEN 10 AND 20">example37.txt</option>
<option value="&quot;value&quot; NOT BETWEEN 10 AND 20">example38-alt01.txt</option>
<option value="NOT &quot;value&quot; BETWEEN 10 AND 20">example38.txt</option>
<option value="&quot;value&quot; IN (1.0, 2.0, 3.0)">example39.txt</option>
<option value="&quot;value&quot; NOT IN ('a', 'b', 'c')">example40-alt01.txt</option>
<option value="NOT &quot;value&quot; IN ('a', 'b', 'c')">example40.txt</option>
<option value="&quot;value&quot; IS NULL">example41.txt</option>
<option value="&quot;value&quot; IS NOT NULL">example42-alt01.txt</option>
<option value="NOT &quot;value&quot; IS NULL">example42.txt</option>
<option value="&quot;name&quot; NOT LIKE 'foo%' AND &quot;value&quot; > 10">example43-alt01.txt</option>
<option value="(NOT &quot;name&quot; LIKE 'foo%' AND &quot;value&quot; > 10)">example43.txt</option>
<option value="&quot;value&quot; IS NULL OR &quot;value&quot; BETWEEN 10 AND 20">example44-alt01.txt</option>
<option value="(&quot;value&quot; IS NULL OR &quot;value&quot; BETWEEN 10 AND 20)">example44.txt</option>
<option value="S_INTERSECTS(&quot;geometry&quot;, BBOX(-128.098193, -1.1, -99999.0, 180.0, 90.0, 100000.0))">example45.txt</option>
<option value="S_EQUALS(
    POLYGON (
        (-0.333333 89.0, -102.723546 -0.5, -179.0 -89.0, -1.9 89.0, -0.0 89.0, 2.00001 -1.9, -0.333333 89.0)
    ),
    &quot;geometry&quot;
)">example46-alt01.txt</option>
<option value="S_EQUALS(POLYGON ((-0.333333 89.0, -102.723546 -0.5, -179.0 -89.0, -1.9 89.0, -0.0 89.0, 2.00001 -1.9, -0.333333 89.0)), &quot;geometry&quot;)">example46.txt</option>
<option value="S_DISJOINT(&quot;geometry&quot;, MULTIPOLYGON (((144.022387 45.176126, -1.1 0.0, 180.0 47.808086, 144.022387 45.176126))))">example47.txt</option>
<option value="S_TOUCHES(&quot;geometry&quot;, MULTILINESTRING ((-1.9 -0.99999, 75.292574 1.5, -0.5 -4.016458, -31.708594 -74.743801, 179.0 -90.0),(-1.9 -1.1, 1.5 8.547371)))">example48.txt</option>
<option value="S_WITHIN(POLYGON ((-49.88024 0.5 -75993.341684, -1.5 -0.99999 -100000.0, 0.0 0.5 -0.333333, -49.88024 0.5 -75993.341684), (-65.887123 2.00001 -100000.0, 0.333333 -53.017711 -79471.332949, 180.0 0.0 1852.616704, -65.887123 2.00001 -100000.0)), &quot;geometry&quot;)">example49-alt01.txt</option>
<option value="S_WITHIN(POLYGON Z ((-49.88024 0.5 -75993.341684, -1.5 -0.99999 -100000.0, 0.0 0.5 -0.333333, -49.88024 0.5 -75993.341684), (-65.887123 2.00001 -100000.0, 0.333333 -53.017711 -79471.332949, 180.0 0.0 1852.616704, -65.887123 2.00001 -100000.0)), &quot;geometry&quot;)">example49.txt</option>
<option value="S_OVERLAPS(&quot;geometry&quot;, BBOX(-179.912109, 1.9, 180.0, 16.897016))">example50.txt</option>
<option value="S_CROSSES(&quot;geometry&quot;, LINESTRING (172.03086 1.5, 1.1 -90.0, -159.757695 0.99999, -180.0 0.5, -12.111235 81.336403, -0.5 64.43958, 0.0 81.991815, -155.93831 90.0))">example51.txt</option>
<option value="S_CONTAINS(&quot;geometry&quot;, POINT (-3.508362 -1.754181))">example52.txt</option>
<option value="T_AFTER(&quot;updated_at&quot;, DATE('2010-02-10'))">example53.txt</option>
<option value="T_BEFORE(updated_at, TIMESTAMP('2012-08-10T05:30:00Z'))">example54-alt01.txt</option>
<option value="T_BEFORE(&quot;updated_at&quot;, TIMESTAMP('2012-08-10T05:30:00.000000Z'))">example54.txt</option>
<option value="T_CONTAINS(INTERVAL('2000-01-01T00:00:00Z', '2005-01-10T01:01:01.393216Z'), INTERVAL(starts_at, ends_at))">example55-alt01.txt</option>
<option value="T_CONTAINS(INTERVAL('2000-01-01T00:00:00.000000Z', '2005-01-10T01:01:01.393216Z'), INTERVAL(starts_at, ends_at))">example55.txt</option>
<option value="T_DISJOINT(INTERVAL('..', '2005-01-10T01:01:01.393216Z'), INTERVAL(starts_at, ends_at))">example56.txt</option>
<option value="T_DURING(INTERVAL(starts_at, ends_at), INTERVAL('2005-01-10', '2010-02-10'))">example57.txt</option>
<option value="T_EQUALS(&quot;updated_at&quot;, DATE('1851-04-29'))">example58.txt</option>
<option value="T_FINISHEDBY(INTERVAL(starts_at, ends_at), INTERVAL('1991-10-07T08:21:06.393262Z', '2010-02-10T05:29:20.073225Z'))">example59.txt</option>
<option value="T_FINISHES(INTERVAL(starts_at, ends_at), INTERVAL('1991-10-07', '2010-02-10T05:29:20.073225Z'))">example60.txt</option>
<option value="T_INTERSECTS(INTERVAL(starts_at, ends_at), INTERVAL('1991-10-07T08:21:06.393262Z', '2010-02-10T05:29:20.073225Z'))">example61.txt</option>
<option value="T_MEETS(INTERVAL('2005-01-10', '2010-02-10'), INTERVAL(starts_at, ends_at))">example62.txt</option>
<option value="T_METBY(INTERVAL('2010-02-10T05:29:20.073225Z', '2010-10-07'), INTERVAL(starts_at, ends_at))">example63.txt</option>
<option value="T_OVERLAPPEDBY(INTERVAL('1991-10-07T08:21:06.393262Z', '2010-02-10T05:29:20.073225Z'), INTERVAL(starts_at, ends_at))">example64.txt</option>
<option value="T_OVERLAPS(INTERVAL(starts_at, ends_at), INTERVAL('1991-10-07T08:21:06.393262Z', '1992-10-09T08:08:08.393473Z'))">example65.txt</option>
<option value="T_STARTEDBY(INTERVAL('1991-10-07T08:21:06.393262Z', '2010-02-10T05:29:20.073225Z'), INTERVAL(starts_at, ends_at))">example66.txt</option>
<option value="T_STARTS(INTERVAL(starts_at, ends_at), INTERVAL('1991-10-07T08:21:06.393262Z', '..'))">example67.txt</option>
<option value="Foo(&quot;geometry&quot;) = TRUE">example68.txt</option>
<option value="FALSE <> Bar(&quot;geometry&quot;, 100, 'a', 'b', FALSE)">example69.txt</option>
<option value="ACCENTI(&quot;owner&quot;) = ACCENTI('Beyoncé')">example70.txt</option>
<option value="CASEI(&quot;owner&quot;) = CASEI('somebody else')">example71.txt</option>
<option value="&quot;value&quot; > (&quot;foo&quot; + 10)">example72.txt</option>
<option value="&quot;value&quot; < (&quot;foo&quot; - 10)">example73.txt</option>
<option value="&quot;value&quot; <> (22.1 * &quot;foo&quot;)">example74.txt</option>
<option value="&quot;value&quot; = (2 / &quot;foo&quot;)">example75.txt</option>
<option value="&quot;value&quot; <= (2 ^ &quot;foo&quot;)">example76.txt</option>
<option value="0 = (&quot;foo&quot; % 2)">example77.txt</option>
<option value="1 = (&quot;foo&quot; div 2)">example78.txt</option>
<option value="A_CONTAINEDBY(&quot;values&quot;, ('a', 'b', 'c'))">example79.txt</option>
<option value="A_CONTAINS(&quot;values&quot;, ('a', 'b', 'c'))">example80.txt</option>
<option value="A_EQUALS(('a', TRUE, 1.0, 8), &quot;values&quot;)">example81.txt</option>
<option value="A_OVERLAPS(&quot;values&quot;, (TIMESTAMP('2012-08-10T05:30:00.000000Z'), DATE('2010-02-10'), FALSE))">example82.txt</option>
<option value="S_EQUALS(MULTIPOINT ((180.0 -0.5), (179.0 -47.121701), (180.0 -0.0), (33.470475 -0.99999), (179.0 -15.333062)), &quot;geometry&quot;)">example83.txt</option>
<option value="S_EQUALS(GEOMETRYCOLLECTION (POINT (1.9 2.00001), POINT (0.0 -2.00001), MULTILINESTRING ((-2.00001 -0.0, -77.292642 -0.5, -87.515626 -0.0, -180.0 12.502773, 21.204842 -1.5, -21.878857 -90.0)), POINT (1.9 0.5), LINESTRING (179.0 1.179148, -148.192487 -65.007816, 0.5 0.333333)), &quot;geometry&quot;)">example84.txt</option>
<option value="value = - foo * 2.0 + &quot;bar&quot; / 6.1234 - &quot;x&quot; ^ 2.0">example85-alt01.txt</option>
<option value="&quot;value&quot; = ((((-1 * &quot;foo&quot;) * 2.0) + (&quot;bar&quot; / 6.1234)) - (&quot;x&quot; ^ 2.0))">example85.txt</option>
<option value="&quot;name&quot; LIKE CASEI('FOO%')">example86.txt</option>
<option value='{"op":"avg","args":[{"property":"windSpeed"}]}'>clause6_01.json</option>
<option value='{"op":"=","args":[{"property":"city"},"Toronto"]}'>clause6_02a.json</option>
<option value='{"op":"<","args":[{"op":"avg","args":[{"property":"windSpeed"}]},4]}'>clause6_02b.json</option>
<option value='{"op":">","args":[{"op":"-","args":[{"property":"balance"},150.0]},0]}'>clause6_02c.json</option>
<option value='{"op":">=","args":[{"property":"updated"},{"date":"1970-01-01"}]}'>clause6_02d.json</option>
<option value='{"op":"not","args":[{"op":"isNull","args":[{"property":"geometry"}]}]}'>clause6_03.json</option>
<option value='{"op":"like","args":[{"property":"name"},"Smith%"]}'>clause7_01.json</option>
<option value='{"op":"between","args":[{"property":"depth"},100.0,150.0]}'>clause7_02.json</option>
<option value='{"op":"in","args":[{"property":"cityName"},["Toronto","Frankfurt","Tokyo","New York"]]}'>clause7_03a.json</option>
<option value='{"op":"not","args":[{"op":"in","args":[{"property":"category"},[1,2,3,4]]}]}'>clause7_03b.json</option>
<option value='{"op":"in","args":[{"op":"casei","args":[{"property":"road_class"}]},[{"op":"casei","args":["Οδος"]},{"op":"casei","args":["Straße"]}]]}'>clause7_04.json</option>
<option value='{"op":"=","args":[{"op":"accenti","args":[{"property":"etat_vol"}]},{"op":"accenti","args":["débárquér"]}]}'>clause7_05.json</option>
<option value='{"op":"s_intersects","args":[{"property":"geometry"},{"type":"Point","coordinates":[36.319836,32.288087]}]}'>clause7_07.json</option>
<option value='{"op":"s_crosses","args":[{"property":"road"},{"type":"Polygon","coordinates":[[[43.7286,-79.2986],[43.7311,-79.2996],[43.7323,-79.2972],[43.7326,-79.2971],[43.7350,-79.2981],[43.7350,-79.2982],[43.7352,-79.2982],[43.7357,-79.2956],[43.7337,-79.2948],[43.7343,-79.2933],[43.7339,-79.2923],[43.7327,-79.2947],[43.7320,-79.2942],[43.7322,-79.2937],[43.7306,-79.2930],[43.7303,-79.2930],[43.7299,-79.2928],[43.7286,-79.2986]]]}]}'>clause7_10.json</option>
<option value='{"op":"t_intersects","args":[{"property":"event_time"},{"interval":["1969-07-16T05:32:00Z","1969-07-24T16:50:35Z"]}]}'>clause7_12.json</option>
<option value='{"op":"t_during","args":[{"interval":[{"property":"touchdown"},{"property":"liftOff"}]},{"interval":["1969-07-16T13:32:00Z","1969-07-24T16:50:35Z"]}]}'>clause7_13.json</option>
<option value='{"op":"a_contains","args":[{"property":"layer:ids"},["layers-ca","layers-us"]]}'>clause7_15.json</option>
<option value='{"op":"s_crosses","args":[{"type":"LineString","coordinates":[[43.72992,-79.2998],[43.73005,-79.2991],[43.73006,-79.2984],[43.73140,-79.2956],[43.73259,-79.2950],[43.73266,-79.2945],[43.73320,-79.2936],[43.73378,-79.2936],[43.73486,-79.2917]]},{"type":"Polygon","coordinates":[[[43.7286,-79.2986],[43.7311,-79.2996],[43.7323,-79.2972],[43.7326,-79.2971],[43.7350,-79.2981],[43.7350,-79.2982],[43.7352,-79.2982],[43.7357,-79.2956],[43.7337,-79.2948],[43.7343,-79.2933],[43.7339,-79.2923],[43.7327,-79.2947],[43.7320,-79.2942],[43.7322,-79.2937],[43.7306,-79.2930],[43.7303,-79.2930],[43.7299,-79.2928],[43.7286,-79.2986]]]}]}'>clause7_16.json</option>
<option value='{"op":"t_during","args":[{"interval":["1969-07-20T20:17:40Z","1969-07-21T17:54:00Z"]},{"interval":["1969-07-16T13:32:00Z","1969-07-24T16:50:35Z"]}]}'>clause7_17.json</option>
<option value='{"op":"s_within","args":[{"property":"road"},{"op":"Buffer","args":[{"property":"geometry"},10,"m"]}]}'>clause7_18.json</option>
<option value='{"op":">","args":[{"property":"vehicle_height"},{"op":"-","args":[{"property":"bridge_clearance"},1]}]}'>clause7_19.json</option>
<option value='{"op":"=","args":[{"property":"landsat:scene_id"},"LC82030282019133LGN00"]}'>example01.json</option>
<option value='{"op":"like","args":[{"property":"eo:instrument"},"OLI%"]}'>example02.json</option>
<option value='{"op":"in","args":[{"property":"landsat:wrs_path"},["153","154","15X"]]}'>example03.json</option>
<option value='{"op":"and","args":[{"op":"<","args":[{"property":"eo:cloud_cover"},0.1]},{"op":"=","args":[{"property":"landsat:wrs_row"},28]},{"op":"=","args":[{"property":"landsat:wrs_path"},203]}]}'>example04.json</option>
<option value='{"op":"or","args":[{"op":"=","args":[{"property":"eo:cloud_cover"},0.1]},{"op":"=","args":[{"property":"eo:cloud_cover"},0.2]}]}'>example05a.json</option>
<option value='{"op":"in","args":[{"property":"eo:cloud_cover"},[0.1,0.2]]}'>example05b.json</option>
<option value='{"op":"and","args":[{"op":"between","args":[{"property":"eo:cloud_cover"},0.1,0.2]},{"op":"=","args":[{"property":"landsat:wrs_row"},28]},{"op":"=","args":[{"property":"landsat:wrs_path"},203]}]}'>example06a.json</option>
<option value='{"op":"and","args":[{"op":">=","args":[{"property":"eo:cloud_cover"},0.1]},{"op":"<=","args":[{"property":"eo:cloud_cover"},0.2]},{"op":"=","args":[{"property":"landsat:wrs_row"},28]},{"op":"=","args":[{"property":"landsat:wrs_path"},203]}]}'>example06b.json</option>
<option value='{"op":"and","args":[{"op":"like","args":[{"property":"eo:instrument"},"OLI%"]},{"op":"s_intersects","args":[{"property":"footprint"},{"type":"Polygon","coordinates":[[[43.5845,-79.5442],[43.6079,-79.4893],[43.5677,-79.4632],[43.6129,-79.3925],[43.6223,-79.3238],[43.6576,-79.3163],[43.7945,-79.1178],[43.8144,-79.1542],[43.8555,-79.1714],[43.7509,-79.639],[43.5845,-79.5442]]]}]}]}'>example07.json</option>
<option value='{"op":"and","args":[{"op":"=","args":[{"property":"beamMode"},"ScanSAR Narrow"]},{"op":"=","args":[{"property":"swathDirection"},"ascending"]},{"op":"=","args":[{"property":"polarization"},"HH+VV+HV+VH"]},{"op":"s_intersects","args":[{"property":"footprint"},{"type":"Polygon","coordinates":[[[-77.117938,38.936860],[-77.040604,39.995648],[-76.910536,38.892912],[-77.039359,38.791753],[-77.047906,38.841462],[-77.034183,38.840655],[-77.033142,38.857490],[-77.117938,38.936860]]]}]}]}'>example08.json</option>
<option value='{"op":">","args":[{"property":"floors"},5]}'>example09.json</option>
<option value='{"op":"<=","args":[{"property":"taxes"},500]}'>example10.json</option>
<option value='{"op":"like","args":[{"property":"owner"},"%Jones%"]}'>example11.json</option>
<option value='{"op":"like","args":[{"property":"owner"},"Mike%"]}'>example12.json</option>
<option value='{"op":"not","args":[{"op":"like","args":[{"property":"owner"},"%Mike%"]}]}'>example13.json</option>
<option value='{"op":"=","args":[{"property":"swimming_pool"},true]}'>example14.json</option>
<option value='{"op":"and","args":[{"op":">","args":[{"property":"floors"},5]},{"op":"=","args":[{"property":"swimming_pool"},true]}]}'>example15.json</option>
<option value='{"op":"and","args":[{"op":"=","args":[{"property":"swimming_pool"},true]},{"op":"or","args":[{"op":">","args":[{"property":"floors"},5]},{"op":"like","args":[{"property":"material"},"brick%"]},{"op":"like","args":[{"property":"material"},"%brick"]}]}]}'>example16.json</option>
<option value='{"op":"or","args":[{"op":"and","args":[{"op":">","args":[{"property":"floors"},5]},{"op":"=","args":[{"property":"material"},"brick"]}]},{"op":"=","args":[{"property":"swimming_pool"},true]}]}'>example17.json</option>
<option value='{"op":"or","args":[{"op":"not","args":[{"op":"<","args":[{"property":"floors"},5]}]},{"op":"=","args":[{"property":"swimming_pool"},true]}]}'>example18.json</option>
<option value='{"op":"and","args":[{"op":"or","args":[{"op":"like","args":[{"property":"owner"},"mike%"]},{"op":"like","args":[{"property":"owner"},"Mike%"]}]},{"op":"<","args":[{"property":"floors"},4]}]}'>example19.json</option>
<option value='{"op":"t_before","args":[{"property":"built"},{"date":"2015-01-01"}]}'>example20.json</option>
<option value='{"op":"t_after","args":[{"property":"built"},{"date":"2012-06-05"}]}'>example21.json</option>
<option value='{"op":"t_during","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["2017-06-10T07:30:00Z","2017-06-11T10:30:00Z"]}]}'>example22.json</option>
<option value='{"op":"s_within","args":[{"property":"location"},{"bbox":[-118,33.8,-117.9,34]}]}'>example23.json</option>
<option value='{"op":"s_intersects","args":[{"property":"geometry"},{"type":"Polygon","coordinates":[[[-10,-10],[10,-10],[10,10],[-10,-10]]]}]}'>example24.json</option>
<option value='{"op":"and","args":[{"op":">","args":[{"property":"floors"},5]},{"op":"s_within","args":[{"property":"geometry"},{"bbox":[-118,33.8,-117.9,34]}]}]}'>example25.json</option>
<option value='{"op":"in","args":[{"op":"casei","args":[{"property":"road_class"}]},[{"op":"casei","args":["Οδος"]},{"op":"casei","args":["Straße"]}]]}'>example26.json</option>
<option value='{"op":"=","args":[{"op":"accenti","args":[{"property":"etat_vol"}]},{"op":"accenti","args":["débárquér"]}]}'>example27.json</option>
<option value='{"op":"like","args":[{"op":"casei","args":[{"property":"geophys:SURVEY_NAME"}]},{"op":"casei","args":["%calcutta%"]}]}'>example28.json</option>
<option value='{"op":"=","args":[{"property":"id"},"fa7e1920-9107-422d-a3db-c468cbc5d6df"]}'>example29.json</option>
<option value='{"op":"<>","args":[{"property":"id"},"fa7e1920-9107-422d-a3db-c468cbc5d6df"]}'>example30.json</option>
<option value='{"op":"<","args":[{"property":"value"},10]}'>example31.json</option>
<option value='{"op":">","args":[{"property":"value"},10]}'>example32.json</option>
<option value='{"op":"<=","args":[{"property":"value"},10]}'>example33.json</option>
<option value='{"op":">=","args":[{"property":"value"},10]}'>example34.json</option>
<option value='{"op":"like","args":[{"property":"name"},"foo%"]}'>example35.json</option>
<option value='{"op":"not","args":[{"op":"like","args":[{"property":"name"},"foo%"]}]}'>example36.json</option>
<option value='{"op":"between","args":[{"property":"value"},10,20]}'>example37.json</option>
<option value='{"op":"not","args":[{"op":"between","args":[{"property":"value"},10,20]}]}'>example38.json</option>
<option value='{"op":"in","args":[{"property":"value"},[1.0,2.0,3.0]]}'>example39.json</option>
<option value='{"op":"not","args":[{"op":"in","args":[{"property":"value"},["a","b","c"]]}]}'>example40.json</option>
<option value='{"op":"isNull","args":[{"property":"value"}]}'>example41.json</option>
<option value='{"op":"not","args":[{"op":"isNull","args":[{"property":"value"}]}]}'>example42.json</option>
<option value='{"op":"and","args":[{"op":"not","args":[{"op":"like","args":[{"property":"name"},"foo%"]}]},{"op":">","args":[{"property":"value"},10]}]}'>example43.json</option>
<option value='{"op":"or","args":[{"op":"isNull","args":[{"property":"value"}]},{"op":"between","args":[{"property":"value"},10,20]}]}'>example44.json</option>
<option value='{"op":"s_intersects","args":[{"property":"geometry"},{"bbox":[-128.098193,-1.1,-99999.0,180.0,90.0,100000.0]}]}'>example45.json</option>
<option value='{"op":"s_equals","args":[{"type":"Polygon","coordinates":[[[-0.333333,89.0],[-102.723546,-0.5],[-179.0,-89.0],[-1.9,89.0],[-0.0,89.0],[2.00001,-1.9],[-0.333333,89.0]]]},{"property":"geometry"}]}'>example46.json</option>
<option value='{"op":"s_disjoint","args":[{"property":"geometry"},{"type":"MultiPolygon","coordinates":[[[[144.022387,45.176126],[-1.1,0.0],[180.0,47.808086],[144.022387,45.176126]]]]}]}'>example47.json</option>
<option value='{"op":"s_touches","args":[{"property":"geometry"},{"type":"MultiLineString","coordinates":[[[-1.9,-0.99999],[75.292574,1.5],[-0.5,-4.016458],[-31.708594,-74.743801],[179.0,-90.0]],[[-1.9,-1.1],[1.5,8.547371]]]}]}'>example48.json</option>
<option value='{"op":"s_within","args":[{"type":"Polygon","coordinates":[[[-49.88024,0.5,-75993.341684],[-1.5,-0.99999,-100000.0],[0.0,0.5,-0.333333],[-49.88024,0.5,-75993.341684]],[[-65.887123,2.00001,-100000.0],[0.333333,-53.017711,-79471.332949],[180.0,0.0,1852.616704],[-65.887123,2.00001,-100000.0]]]},{"property":"geometry"}]}'>example49.json</option>
<option value='{"op":"s_overlaps","args":[{"property":"geometry"},{"bbox":[-179.912109,1.9,180.0,16.897016]}]}'>example50.json</option>
<option value='{"op":"s_crosses","args":[{"property":"geometry"},{"type":"LineString","coordinates":[[172.03086,1.5],[1.1,-90.0],[-159.757695,0.99999],[-180.0,0.5],[-12.111235,81.336403],[-0.5,64.43958],[0.0,81.991815],[-155.93831,90.0]]}]}'>example51.json</option>
<option value='{"op":"s_contains","args":[{"property":"geometry"},{"type":"Point","coordinates":[-3.508362,-1.754181]}]}'>example52.json</option>
<option value='{"op":"t_after","args":[{"property":"updated_at"},{"date":"2010-02-10"}]}'>example53.json</option>
<option value='{"op":"t_before","args":[{"property":"updated_at"},{"timestamp":"2012-08-10T05:30:00Z"}]}'>example54.json</option>
<option value='{"op":"t_contains","args":[{"interval":["2000-01-01T00:00:00Z","2005-01-10T01:01:01.393216Z"]},{"interval":[{"property":"starts_at"},{"property":"ends_at"}]}]}'>example55.json</option>
<option value='{"op":"t_disjoint","args":[{"interval":["..","2005-01-10T01:01:01.393216Z"]},{"interval":[{"property":"starts_at"},{"property":"ends_at"}]}]}'>example56.json</option>
<option value='{"op":"t_during","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["2005-01-10","2010-02-10"]}]}'>example57.json</option>
<option value='{"op":"t_equals","args":[{"property":"updated_at"},{"date":"1851-04-29"}]}'>example58.json</option>
<option value='{"op":"t_finishedBy","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["1991-10-07T08:21:06.393262Z","2010-02-10T05:29:20.073225Z"]}]}'>example59.json</option>
<option value='{"op":"t_finishes","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["1991-10-07","2010-02-10T05:29:20.073225Z"]}]}'>example60.json</option>
<option value='{"op":"t_intersects","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["1991-10-07T08:21:06.393262Z","2010-02-10T05:29:20.073225Z"]}]}'>example61.json</option>
<option value='{"op":"t_meets","args":[{"interval":["2005-01-10","2010-02-10"]},{"interval":[{"property":"starts_at"},{"property":"ends_at"}]}]}'>example62.json</option>
<option value='{"op":"t_metBy","args":[{"interval":["2010-02-10T05:29:20.073225Z","2010-10-07"]},{"interval":[{"property":"starts_at"},{"property":"ends_at"}]}]}'>example63.json</option>
<option value='{"op":"t_overlappedBy","args":[{"interval":["1991-10-07T08:21:06.393262Z","2010-02-10T05:29:20.073225Z"]},{"interval":[{"property":"starts_at"},{"property":"ends_at"}]}]}'>example64.json</option>
<option value='{"op":"t_overlaps","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["1991-10-07T08:21:06.393262Z","1992-10-09T08:08:08.393473Z"]}]}'>example65.json</option>
<option value='{"op":"t_startedBy","args":[{"interval":["1991-10-07T08:21:06.393262Z","2010-02-10T05:29:20.073225Z"]},{"interval":[{"property":"starts_at"},{"property":"ends_at"}]}]}'>example66.json</option>
<option value='{"op":"t_starts","args":[{"interval":[{"property":"starts_at"},{"property":"ends_at"}]},{"interval":["1991-10-07T08:21:06.393262Z",".."]}]}'>example67.json</option>
<option value='{"op":"=","args":[{"op":"Foo","args":[{"property":"geometry"}]},true]}'>example68.json</option>
<option value='{"op":"<>","args":[false,{"op":"Bar","args":[{"property":"geometry"},100,"a","b",false]}]}'>example69.json</option>
<option value='{"op":"=","args":[{"op":"accenti","args":[{"property":"owner"}]},{"op":"accenti","args":["Beyoncé"]}]}'>example70.json</option>
<option value='{"op":"=","args":[{"op":"casei","args":[{"property":"owner"}]},{"op":"casei","args":["somebody else"]}]}'>example71.json</option>
<option value='{"op":">","args":[{"property":"value"},{"op":"+","args":[{"property":"foo"},10]}]}'>example72.json</option>
<option value='{"op":"<","args":[{"property":"value"},{"op":"-","args":[{"property":"foo"},10]}]}'>example73.json</option>
<option value='{"op":"<>","args":[{"property":"value"},{"op":"*","args":[22.1,{"property":"foo"}]}]}'>example74.json</option>
<option value='{"op":"=","args":[{"property":"value"},{"op":"/","args":[2,{"property":"foo"}]}]}'>example75.json</option>
<option value='{"op":"<=","args":[{"property":"value"},{"op":"^","args":[2,{"property":"foo"}]}]}'>example76.json</option>
<option value='{"op":"=","args":[0,{"op":"%","args":[{"property":"foo"},2]}]}'>example77.json</option>
<option value='{"op":"=","args":[1,{"op":"div","args":[{"property":"foo"},2]}]}'>example78.json</option>
<option value='{"op":"a_containedBy","args":[{"property":"values"},["a","b","c"]]}'>example79.json</option>
<option value='{"op":"a_contains","args":[{"property":"values"},["a","b","c"]]}'>example80.json</option>
<option value='{"op":"a_equals","args":[["a",true,1.0,8],{"property":"values"}]}'>example81.json</option>
<option value='{"op":"a_overlaps","args":[{"property":"values"},[{"timestamp":"2012-08-10T05:30:00Z"},{"date":"2010-02-10"},false]]}'>example82.json</option>
<option value='{"op":"s_equals","args":[{"type":"MultiPoint","coordinates":[[180.0,-0.5],[179.0,-47.121701],[180.0,-0.0],[33.470475,-0.99999],[179.0,-15.333062]]},{"property":"geometry"}]}'>example83.json</option>
<option value='{"op":"s_equals","args":[{"type":"GeometryCollection","geometries":[{"type":"Point","coordinates":[1.9,2.00001]},{"type":"Point","coordinates":[0.0,-2.00001]},{"type":"MultiLineString","coordinates":[[[-2.00001,-0.0],[-77.292642,-0.5],[-87.515626,-0.0],[-180.0,12.502773],[21.204842,-1.5],[-21.878857,-90.0]]]},{"type":"Point","coordinates":[1.9,0.5]},{"type":"LineString","coordinates":[[179.0,1.179148],[-148.192487,-65.007816],[0.5,0.333333]]}]},{"property":"geometry"}]}'>example84.json</option>
<option value='{"op":"=","args":[{"property":"value"},{"op":"-","args":[{"op":"+","args":[{"op":"*","args":[{"op":"*","args":[-1,{"property":"foo"}]},2.0]},{"op":"/","args":[{"property":"bar"},6.1234]}]},{"op":"^","args":[{"property":"x"},2.0]}]}]}'>example85.json</option>
<option value='{"op":"like","args":[{"property":"name"},{"op":"casei","args":["FOO%"]}]}'>example86.json</option>



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
