#pragma once

#include <cstddef>
#include <QImage>
#include <QQuickItem>

using BrowserFrameCleanup = void (*)(void *);

class BrowserFrameItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(bool hasFrame READ hasFrame NOTIFY hasFrameChanged)

public:
    explicit BrowserFrameItem(QQuickItem *parent = nullptr);
    ~BrowserFrameItem() override;

    bool hasFrame() const;
    void setFrame(
        const uchar *bgra,
        qsizetype length,
        int width,
        int height,
        quint64 epoch,
        void *owner,
        BrowserFrameCleanup cleanup);
    void clearFrame();

signals:
    void hasFrameChanged();

protected:
    QSGNode *updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) override;

private:
    QImage image_;
    quint64 epoch_ = 0;
    quint64 renderedEpoch_ = 0;
};

extern "C" void hunk_register_browser_frame_item();
extern "C" void hunk_browser_frame_publish(
    const unsigned char *bgra,
    size_t length,
    unsigned int width,
    unsigned int height,
    unsigned long long epoch,
    void *owner,
    BrowserFrameCleanup cleanup);
extern "C" void hunk_browser_frame_clear();
