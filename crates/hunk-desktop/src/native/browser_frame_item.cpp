#include "browser_frame_item.h"

#include <limits>
#include <QPointer>
#include <QQmlEngine>
#include <QSGSimpleTextureNode>
#include <QSGTexture>
#include <qqml.h>
#include <rhi/qrhi.h>

namespace {
QPointer<BrowserFrameItem> activeItem;

class BrowserFrameTexture final : public QSGTexture {
public:
    ~BrowserFrameTexture() override {
        delete texture_;
    }

    void setImage(const QImage &image) {
        pendingImage_ = image;
        size_ = image.size();
        dirty_ = true;
    }

    qint64 comparisonKey() const override {
        const void *key = texture_ ? static_cast<const void *>(texture_)
                                   : static_cast<const void *>(this);
        return qint64(reinterpret_cast<quintptr>(key));
    }

    QSize textureSize() const override {
        return size_;
    }

    bool hasAlphaChannel() const override {
        return true;
    }

    bool hasMipmaps() const override {
        return false;
    }

    QRhiTexture *rhiTexture() const override {
        return texture_;
    }

    void commitTextureOperations(
        QRhi *rhi,
        QRhiResourceUpdateBatch *resourceUpdates) override {
        if (!dirty_ || pendingImage_.isNull() || !resourceUpdates)
            return;
        bool needsCreate = false;
        if (!texture_) {
            texture_ = rhi->newTexture(QRhiTexture::BGRA8, size_);
            needsCreate = true;
        } else if (texture_->pixelSize() != size_) {
            texture_->setPixelSize(size_);
            needsCreate = true;
        }
        if (needsCreate && !texture_->create())
            return;
        // QRhi retains an implicitly shared QImage in the upload batch, so resetting the pending
        // handle here does not release the Rust-owned pixels before the upload completes.
        resourceUpdates->uploadTexture(texture_, pendingImage_);
        pendingImage_ = QImage();
        dirty_ = false;
    }

private:
    QImage pendingImage_;
    QSize size_;
    QRhiTexture *texture_ = nullptr;
    bool dirty_ = false;
};
}

BrowserFrameItem::BrowserFrameItem(QQuickItem *parent) : QQuickItem(parent) {
    setFlag(ItemHasContents, true);
    activeItem = this;
}

BrowserFrameItem::~BrowserFrameItem() {
    if (activeItem == this)
        activeItem.clear();
}

bool BrowserFrameItem::hasFrame() const {
    return !image_.isNull();
}

void BrowserFrameItem::setFrame(
    const uchar *bgra,
    qsizetype length,
    int width,
    int height,
    quint64 epoch,
    void *owner,
    BrowserFrameCleanup cleanup) {
    const bool invalidDimensions = width <= 0
        || height <= 0
        || qsizetype(width) > std::numeric_limits<qsizetype>::max() / 4;
    const qsizetype rowBytes = invalidDimensions ? 0 : qsizetype(width) * 4;
    const bool invalidLength = !invalidDimensions
        && qsizetype(height) > std::numeric_limits<qsizetype>::max() / rowBytes;
    const qsizetype expected = invalidDimensions || invalidLength
        ? 0
        : rowBytes * qsizetype(height);
    if (!bgra || invalidDimensions || invalidLength || length != expected || epoch == epoch_) {
        if (cleanup)
            cleanup(owner);
        return;
    }

    const bool hadFrame = hasFrame();
    image_ = QImage(
        bgra,
        width,
        height,
        rowBytes,
        QImage::Format_ARGB32,
        cleanup,
        owner);
    epoch_ = epoch;
    if (!hadFrame)
        emit hasFrameChanged();
    update();
}

void BrowserFrameItem::clearFrame() {
    if (image_.isNull())
        return;
    image_ = QImage();
    epoch_ = 0;
    renderedEpoch_ = 0;
    emit hasFrameChanged();
    update();
}

QSGNode *BrowserFrameItem::updatePaintNode(QSGNode *oldNode, UpdatePaintNodeData *) {
    if (image_.isNull() || !window()) {
        delete oldNode;
        return nullptr;
    }

    auto *node = static_cast<QSGSimpleTextureNode *>(oldNode);
    if (!node) {
        node = new QSGSimpleTextureNode();
        node->setTexture(new BrowserFrameTexture());
        node->setOwnsTexture(true);
        node->setFiltering(QSGTexture::Linear);
    }
    if (renderedEpoch_ != epoch_) {
        static_cast<BrowserFrameTexture *>(node->texture())->setImage(image_);
        node->markDirty(QSGNode::DirtyMaterial);
        renderedEpoch_ = epoch_;
    }
    node->setRect(boundingRect());
    return node;
}

extern "C" void hunk_register_browser_frame_item() {
    qmlRegisterType<BrowserFrameItem>("Hunk.Native", 1, 0, "BrowserFrameItem");
}

extern "C" void hunk_browser_frame_publish(
    const unsigned char *bgra,
    size_t length,
    unsigned int width,
    unsigned int height,
    unsigned long long epoch,
    void *owner,
    BrowserFrameCleanup cleanup) {
    const bool dimensionsFit = width <= unsigned(std::numeric_limits<int>::max())
        && height <= unsigned(std::numeric_limits<int>::max());
    const bool lengthFits = length <= size_t(std::numeric_limits<qsizetype>::max());
    if (activeItem && dimensionsFit && lengthFits) {
        activeItem->setFrame(
            bgra,
            qsizetype(length),
            int(width),
            int(height),
            quint64(epoch),
            owner,
            cleanup);
    } else if (cleanup) {
        cleanup(owner);
    }
}

extern "C" void hunk_browser_frame_clear() {
    if (activeItem)
        activeItem->clearFrame();
}
