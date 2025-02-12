<html>
  <head>
    <meta content="text/html;charset=utf-8" http-equiv="Content-Type"/>
    <script src="https://ajax.googleapis.com/ajax/libs/jquery/3.7.1/jquery.min.js"></script>
  </head>
  <body>
  <script type="module">
    import init, { CQL2 } from './pkg/cql2_wasm.js';

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
        check();
    });

    </script>
    <h1>CQL2 Playground</h1>
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


  </body>
  <script>

  </script>
</html>
