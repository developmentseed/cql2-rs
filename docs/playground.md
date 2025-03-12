<script src="https://ajax.googleapis.com/ajax/libs/jquery/3.7.1/jquery.min.js"></script>
<link href="https://cdnjs.cloudflare.com/ajax/libs/select2/4.0.13/css/select2.min.css" rel="stylesheet" />
<script src="https://cdnjs.cloudflare.com/ajax/libs/select2/4.0.13/js/select2.min.js"></script>

<style>

#cqlin-div {
    height: 200px;
    width: 100%;
    resize: vertical;
    margin-bottom: 0px;
    display: block;
    padding-bottom:0px;
}

.parsed-container {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    margin-top: 0;
    padding-top:0;
}

.parsed-box {
    flex: 1; /* Make both parsed sections take equal space */
    display: flex;
    flex-direction: column;
}

.parsed-box textarea {
    width: 100%;
    height: 200px; /* Limit height */
    resize: vertical;
}

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

      function fetchexamples(){
        fetch('../examples.json')
            .then(response => response.json())
            .then(data => {
                window.examples = data;
                console.log(window.examples);
                set_example_options(data);
                }
            )
            .catch(error => console.error('Error fetching examples json:', error));
      }

      function set_example_options(data){
        $.each(data, function(key, value){
            $('<option/>')
                .val(key)
                .text(value.name)
                .prop('title', value.description)
                .appendTo('#examples');
        });
      }

      function check(from_select=false){
        console.log('check', from_select);
          if (from_select == false){
            $("#examples").val(null).trigger('change');
            $("#example-description").text("");
          }
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
          $("#cql2text").val(txt).css({"background-color": valid ? "#90EE90" : "pink"});
          $("#cql2json").val(jsn).css({"background-color": valid ? "#90EE90" : "pink"});
      }

      $("#cqlin").on('input propertychange', function(){check(false);});


      function example_change(){
          let selectedOption = $('#examples').find(":selected");
          let val = selectedOption.val();
          if (val){

            let textorjson = $("#exampletype").find(":selected");
            let exampletype = textorjson.val();
            let sel = window.examples[val][exampletype];
            let description = selectedOption.attr("title"); // Get the description

            if (sel.startsWith("{")) {
                let j = JSON.parse(sel);
                sel = JSON.stringify(j, null, 2);
            }

            $("#cqlin").val(sel);
            $("#example-description").text("Current example description: " + description); // Set the description above the CQL input
            check(true);
          }
      };

      $("#examples").change(example_change);

      $("#exampletype").change(example_change);

      // Initialize Select2
      $('#examples').select2({
          placeholder: "Search or select an example...",
          allowClear: true,
          width: '100%'
      });

      fetchexamples();
      $("#examples").val('example32').trigger('change');
  });
</script>

<p id="example-description" style="font-weight: margin-bottom: 5px;"></p>

<select id="exampletype" class="searchable-dropdown" >
<option value='cql2_text'>CQL2 Text</option>
<option value='cql2_json'>CQL2 JSON</option>
</select>

<select id="examples" class="searchable-dropdown" >
<option value=''>-</option>

  </select>
  <div id="cqlin-div">
    <textarea id="cqlin" rows="20" cols="114"></textarea>
</div>
  <br/>
  <div class="parsed-container">
    <div class="parsed-box">
        <h3>Parsed CQL2 Text</h3>
        <textarea id="cql2text" rows="15" readonly></textarea>
    </div>
    <div class="parsed-box">
        <h3>Parsed CQL2 JSON</h3>
        <textarea id="cql2json" rows="15" readonly></textarea>
    </div>
</div>
