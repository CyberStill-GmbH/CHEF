package chef;

import burp.api.montoya.BurpExtension;
import burp.api.montoya.MontoyaApi;
import burp.api.montoya.http.message.requests.HttpRequest;
import burp.api.montoya.ui.contextmenu.ContextMenuEvent;
import burp.api.montoya.ui.contextmenu.ContextMenuItemsProvider;
import javax.swing.*;
import java.awt.Component;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

/** Passive selection spike; no scan handlers, outbound requests, listeners, or companion service. */
public final class ChefExtension implements BurpExtension {
    private MontoyaApi api;
    private JTextArea output;
    private final AtomicBoolean unloaded = new AtomicBoolean();
    private final ThreadPoolExecutor worker = new ThreadPoolExecutor(1, 1, 0, TimeUnit.MILLISECONDS,
        new ArrayBlockingQueue<>(1), task -> {
            Thread thread = new Thread(task, "chef-selection"); thread.setDaemon(true); return thread;
        }, new ThreadPoolExecutor.AbortPolicy());

    @Override
    public void initialize(MontoyaApi api) {
        this.api = api;
        api.extension().setName("CHEF — Evidence-backed Attack Surface Correlation (spike)");
        Runnable createUi = () -> {
            output = new JTextArea("CHEF spike: select up to 100 in-scope requests via the context menu.\nNo requests are sent. Raw path/query/auth/body are not retained.");
            output.setEditable(false); output.setLineWrap(true); output.setWrapStyleWord(true);
            api.userInterface().registerSuiteTab("CHEF spike", new JScrollPane(output));
        };
        try {
            if (SwingUtilities.isEventDispatchThread()) createUi.run();
            else SwingUtilities.invokeAndWait(createUi);
        } catch (Exception exception) {
            worker.shutdownNow(); api.logging().logToError("CHEF UI initialization failed"); return;
        }
        api.userInterface().registerContextMenuItemsProvider(new ContextMenuItemsProvider() {
            @Override
            public List<Component> provideMenuItems(ContextMenuEvent event) { return menuItems(event); }
        });
        api.extension().registerUnloadingHandler(() -> {
            unloaded.set(true); worker.shutdownNow();
            SwingUtilities.invokeLater(() -> { if (output != null) output.setText("CHEF unloaded; selection metadata cleared."); });
        });
    }

    private List<Component> menuItems(ContextMenuEvent event) {
        if (unloaded.get() || event.selectedRequestResponses().isEmpty()) return List.of();
        JMenuItem menu = new JMenuItem("CHEF: explain selected in-scope endpoints (spike)");
        menu.addActionListener(action -> {
            if (unloaded.get()) return;
            List<HttpObservation> observations = new ArrayList<>();
            int rejected = 0;
            int count = Math.min(100, event.selectedRequestResponses().size());
            // Copy only bounded normalized metadata; never keep requests in queued jobs or model state.
            for (int index = 0; index < count; index++) {
                try {
                    HttpRequest request = event.selectedRequestResponses().get(index).request();
                    if (!request.isInScope()) { rejected++; continue; }
                    var service = request.httpService();
                    observations.add(HttpObservation.from(request.method(), service.host(), service.port(), service.secure(),
                        request.pathWithoutQuery(), Instant.now(), index));
                } catch (RuntimeException invalid) { rejected++; }
            }
            int rejectedCount = rejected;
            int omittedCount = Math.max(0, event.selectedRequestResponses().size() - count);
            try {
                worker.execute(() -> {
                    try {
                        if (unloaded.get() || Thread.currentThread().isInterrupted()) return;
                        String body = observations.stream().map(HttpObservation::toJson).collect(java.util.stream.Collectors.joining(","));
                        String json = "{\"schemaVersion\":\"1.1.0\",\"observations\":[" + body + "]}";
                        String explanation = "Passive metadata only. Observed endpoints are not confirmed vulnerabilities.\n" +
                            "Rejected: " + rejectedCount + "; omitted above selection limit: " + omittedCount + "\n\n";
                        SwingUtilities.invokeLater(() -> { if (!unloaded.get()) output.setText(explanation + json); });
                    } catch (RuntimeException exception) { api.logging().logToError("CHEF background processing failed; no traffic retained"); }
                });
            } catch (java.util.concurrent.RejectedExecutionException busy) {
                if (!unloaded.get()) output.setText("CHEF queue is full. Wait for the current selection and retry.");
            }
        });
        return List.of(menu);
    }
}
