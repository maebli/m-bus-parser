import io.github.maebli.mbus.MbusParser;

public class Parse {
    public static void main(String[] args) {
        MbusParser parser = new MbusParser();
        System.out.println(parser.parseHex("1040014116"));
    }
}
